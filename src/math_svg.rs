use crate::ast::*;
use indoc::{formatdoc, writedoc};
use itertools::Itertools;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::{self, Command};
use tempdir::TempDir;

fn write_latex(out: &mut impl io::Write, preamble: &[&str], latex: &str) -> Result<(), io::Error> {
    let preamble = preamble
        .iter()
        .copied()
        .format_with("\n", |line, f| f(&format_args!("{}", line)));
    // TODO: Get rid of mathtools here?
    writedoc! {out, r#"
        \documentclass{{minimal}}
        {preamble}
        \usepackage{{mathtools}}
        \mathtoolsset{{showonlyrefs}}
        \begin{{document}}
        {latex}
        \end{{document}}
    "#}
}

#[derive(Debug)]
pub enum LatexToSvgError {
    Io(io::Error),
    PdfLatex(process::Output),
    PdfCrop(process::Output),
    Pdf2Svg(process::Output),
    BadSvg,
}

impl From<io::Error> for LatexToSvgError {
    fn from(err: io::Error) -> LatexToSvgError {
        LatexToSvgError::Io(err)
    }
}

pub fn pdf_latex(tex_file_path: &Path) -> Result<process::Output, io::Error> {
    let mut cmd = Command::new("pdflatex");
    if let Some(parent) = tex_file_path.parent() {
        cmd.current_dir(parent);
    }
    cmd.arg("-interaction=nonstopmode");
    cmd.arg(&tex_file_path);
    let output = cmd.output()?;
    Ok(output)
}

pub enum PreambleDiagnosis<'a> {
    Ok(process::Output),
    OffendingLines(process::Output, &'a [&'a str]),
}

pub fn dummy_pdf_latex(preamble: &[&str]) -> Result<process::Output, io::Error> {
    let dummy_content = "$123$";

    let tmp_dir = TempDir::new("latex-to-html")?;
    let tex_file_path = tmp_dir.path().join("doc.tex");
    let mut tex_file = File::create(&tex_file_path)?;
    write_latex(&mut tex_file, preamble, dummy_content)?;
    pdf_latex(&tex_file_path)
}

pub fn has_even_curly_braces(preamble_part: &[&str]) -> bool {
    let mut open = 0;
    let mut close = 0;
    for c in preamble_part
        .iter()
        .copied()
        .map(|line| line.chars())
        .flatten()
    {
        if c == '{' {
            open += 1;
        }
        if c == '}' {
            close += 1;
        }
    }

    open == close
}

pub fn split_preamble(preamble_part: &[&str]) -> Option<usize> {
    if preamble_part.len() < 2 {
        return None;
    }

    let mut split_index = preamble_part.len() / 2;

    // If the split resulted in an unmatched curly braces in the lower part, reduce the split index
    // until curly braces are matched.
    while split_index > 0 && !has_even_curly_braces(&preamble_part[0..split_index]) {
        split_index -= 1;
    }

    // If we couldn't find a split with matching curly braces, try again but this time increase the
    // split index.
    if split_index == 0 {
        split_index = preamble_part.len() / 2;
        while split_index < preamble_part.len()
            && !has_even_curly_braces(&preamble_part[0..split_index])
        {
            split_index += 1;
        }
        if split_index == preamble_part.len() {
            return None;
        }
    }

    Some(split_index)
}

pub fn diagnose_preamble<'a>(preamble: &'a [&'a str]) -> Result<PreambleDiagnosis<'a>, io::Error> {
    let output = dummy_pdf_latex(preamble)?;
    if output.status.success() {
        return Ok(PreambleDiagnosis::Ok(output));
    }

    let mut known_good = 0;
    let mut known_bad = preamble.len();
    let mut bad_output = output;

    while let Some(split_index) = split_preamble(&preamble[known_good..known_bad]) {
        let split_index = split_index + known_good;
        let output = dummy_pdf_latex(&preamble[0..split_index])?;
        if output.status.success() {
            known_good = split_index;
        } else {
            bad_output = output;
            known_bad = split_index;
        }
    }

    Ok(PreambleDiagnosis::OffendingLines(
        bad_output,
        &preamble[known_good..known_bad],
    ))
}

pub fn latex_to_svg(preamble: &[&str], latex: &str) -> Result<String, LatexToSvgError> {
    let tmp_dir = TempDir::new("latex-to-html")?;

    let tex_file_path = tmp_dir.path().join("doc.tex");
    let pdf_file_path = tmp_dir.path().join("doc.pdf");
    let pdf_crop_file_path = tmp_dir.path().join("doc-crop.pdf");
    let svg_file_path = tmp_dir.path().join("doc.svg");

    let mut tex_file = File::create(&tex_file_path).map_err(LatexToSvgError::Io)?;
    write_latex(&mut tex_file, preamble, latex)?;

    let pdf_latex_output = pdf_latex(&tex_file_path)?;
    if !pdf_latex_output.status.success() {
        return Err(LatexToSvgError::PdfLatex(pdf_latex_output));
    }

    let mut pdf_crop_cmd = Command::new("pdfcrop");
    pdf_crop_cmd.current_dir(tmp_dir.path());
    pdf_crop_cmd.arg(&pdf_file_path);
    pdf_crop_cmd.arg(&pdf_crop_file_path);
    let pdf_crop_output = pdf_crop_cmd.output()?;
    if !pdf_crop_output.status.success() {
        return Err(LatexToSvgError::PdfCrop(pdf_crop_output));
    }

    let mut pdf2svg_cmd = Command::new("pdf2svg");
    pdf2svg_cmd.current_dir(tmp_dir.path());
    pdf2svg_cmd.arg(&pdf_crop_file_path);
    pdf2svg_cmd.arg(&svg_file_path);
    let pdf2svg_output = pdf2svg_cmd.output()?;
    if !pdf2svg_output.status.success() {
        return Err(LatexToSvgError::Pdf2Svg(pdf2svg_output));
    }

    let svg = std::fs::read_to_string(&svg_file_path)?;
    Ok(svg)
}

pub struct SvgInfo {
    pub width_em: f64,
    pub height_em: f64,
    pub baseline_em: Option<f64>,
}

// Converts the dimensions of the svg from pt to em. Returns (width, height) in em.
pub fn svg_dimensions_to_em(svg: &mut minidom::Element) -> Result<(f64, f64), LatexToSvgError> {
    let bad_svg = || LatexToSvgError::BadSvg;

    let width_attr = svg.attr("width").ok_or(bad_svg())?;
    let width_pt: f64 = width_attr
        .strip_suffix("pt")
        .ok_or(bad_svg())?
        .parse()
        .map_err(|_| bad_svg())?;
    let width_em = width_pt / 10.0;

    let height_attr = svg.attr("height").ok_or(bad_svg())?;
    let height_pt: f64 = height_attr
        .strip_suffix("pt")
        .ok_or(bad_svg())?
        .parse()
        .map_err(|_| bad_svg())?;
    let height_em = height_pt / 10.0;

    svg.set_attr("width", format!("{width_em}em"));
    svg.set_attr("height", format!("{height_em}em"));

    Ok((width_em, height_em))
}

// Removes the baseline point from the svg. Returns the y coordinate of the center of the point,
// i.e. the y-coordinate that corresponds to the baseline.
pub fn remove_baseline_point(svg_el: &mut minidom::Element) -> Result<f64, LatexToSvgError> {
    let bad_svg = || LatexToSvgError::BadSvg;

    let g_el: &mut minidom::element::Element = svg_el
        .get_child_mut("g", minidom::NSChoice::Any)
        .ok_or(bad_svg())?;
    if g_el.attr("id") != Some("surface1") {
        return Err(LatexToSvgError::BadSvg);
    }

    let path_el = g_el
        .remove_child("path", minidom::NSChoice::Any)
        .ok_or(bad_svg())?;
    let transform_attr = path_el.attr("transform").ok_or(bad_svg())?;

    let y_substr_begin = 1 + transform_attr.rfind(",").ok_or(bad_svg())?;
    let y_substr_end = transform_attr.rfind(")").ok_or(bad_svg())?;
    let y_str = &transform_attr[y_substr_begin..y_substr_end];

    let y: f64 = y_str.parse().map_err(|_| bad_svg())?;

    let baseline_em = (y + 0.5) / 10.0;
    Ok(baseline_em)
}

pub fn math_to_svg(
    preamble: &[&str],
    math: &Math,
) -> Result<(minidom::Element, SvgInfo), LatexToSvgError> {
    use Math::*;
    let latex = match math {
        Inline(content) => {
            formatdoc! {r#"
                    $\makebox[0pt][l]{{\rule{{1pt}}{{1pt}}}}{content}$
                "#}
        }
        Display { source, .. } | Mathpar { source, .. } => source.to_string(),
    };

    let svg = latex_to_svg(preamble, &latex)?;
    let bad_svg = || LatexToSvgError::BadSvg;
    let mut svg_el: minidom::Element = svg.parse().map_err(|_| bad_svg())?;
    let (width_em, height_em) = svg_dimensions_to_em(&mut svg_el)?;

    let baseline_em = match math {
        Inline(_) => Some(remove_baseline_point(&mut svg_el)?),
        Display { .. } | Mathpar { .. } => None,
    };

    Ok((
        svg_el,
        SvgInfo {
            width_em,
            height_em,
            baseline_em,
        },
    ))
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathDigest(pub [u8; 32]);

impl Display for MathDigest {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        write!(out, "{}", hex::encode(self.0))?;
        Ok(())
    }
}

pub fn hash_math(preamble: &[&str], math: &Math) -> MathDigest {
    let mut hasher = Sha256::new();

    for line in preamble {
        hasher.update(line.as_bytes());
    }

    use Math::*;
    match math {
        Inline(source) => {
            hasher.update(&[0]);
            hasher.update(source);
        }
        Display { source, label: _ } => {
            hasher.update(&[1]);
            hasher.update(source);
        }
        Mathpar { source, label: _ } => {
            hasher.update(&[2]);
            hasher.update(source);
        }
    }

    MathDigest(hasher.finalize().as_slice().try_into().unwrap())
}

pub const SVG_OUT_DIR: &'static str = "img-math";

pub fn emit_math_svg_files<'a, 'b>(
    out_dir: &'a Path,
    preamble: &'b [&'b str],
    math: &[&'b Math<'b>],
) -> Result<(), (&'b Math<'b>, LatexToSvgError)> {
    let out_dir = out_dir.join(SVG_OUT_DIR);
    fs::create_dir_all(&out_dir).unwrap();

    // Collect all math nodes that need to be compiled. There may be duplicate new math nodes;
    // these need to be compiled only once. We detect duplicates by saving digests in a hash set.
    let mut old_math_digests: HashSet<MathDigest> = HashSet::new();
    let new_math: Vec<&'b Math<'b>> = math
        .iter()
        .copied()
        .filter(|math| {
            let digest = hash_math(preamble, &math);
            let svg_path = out_dir.join(&format!("{digest}.svg"));
            let is_new = !old_math_digests.contains(&digest) && !svg_path.exists();
            old_math_digests.insert(digest);
            is_new
        })
        .collect();

    // If we have a lot of math to compile, check whether the preamble is OK first.
    if new_math.len() > 16 {
        let first_math = *new_math.first().unwrap();
        let output =
            dummy_pdf_latex(preamble).map_err(|err| (first_math, LatexToSvgError::Io(err)))?;
        if !output.status.success() {
            return Err((first_math, LatexToSvgError::PdfLatex(output)));
        }
    }

    // Compile math nodes to svgs in parallel. We write to temporary files first and rename later
    // for two reasons:
    // - To ensure consistency via an atomic rename.
    // - To ensure that we have writting geometry information to the css file if the svg file
    //   exists.
    let new_infos: Vec<Result<SvgInfo, LatexToSvgError>> = new_math
        .par_iter()
        .copied()
        .map(|math| {
            let digest = hash_math(preamble, &math);
            let svg_path_tmp = out_dir.join(&format!("{digest}.svg.tmp"));

            let (svg, svg_info) = math_to_svg(preamble, math)?;
            fs::write(&svg_path_tmp, &String::from(&svg)).unwrap();
            Ok(svg_info)
        })
        .collect();

    // Open the css file containing geometry information about the svgs. We append if it already
    // exists and create otherwise.
    let geometry_path = out_dir.join("geometry.css");
    let mut geometry_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(geometry_path)
        .unwrap();

    // Write geometry info for new math svgs to the css file.
    for (math, svg_info) in new_math.iter().copied().zip(new_infos.iter()) {
        let SvgInfo {
            width_em,
            height_em,
            baseline_em,
        } = match svg_info {
            Ok(svg_info) => svg_info,
            Err(_) => {
                continue;
            }
        };

        let top_em = match baseline_em {
            None => 0.0,
            Some(baseline_em) => height_em - baseline_em,
        };

        let digest = hash_math(preamble, &math);

        writedoc! {geometry_file, r#"
            img[src$="{digest}.svg"] {{
                width: {width_em}em;
                height: {height_em}em;
                top: {top_em}em;
            }}
        "#}
        .unwrap();
    }
    geometry_file.sync_data().unwrap();

    // Rename temporary svg files.
    for (math, svg_info) in new_math.iter().copied().zip(new_infos.iter()) {
        if svg_info.is_err() {
            continue;
        }

        let digest = hash_math(preamble, &math);
        let svg_path = out_dir.join(&format!("{digest}.svg"));
        let svg_path_tmp = out_dir.join(&format!("{digest}.svg.tmp"));

        fs::rename(svg_path_tmp, svg_path).unwrap();
    }

    // Return the first error, if any.
    for (math, svg_info) in new_math.iter().copied().zip(new_infos) {
        if let Err(err) = svg_info {
            return Err((math, err));
        }
    }

    Ok(())
}

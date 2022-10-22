use crate::ast::*;
use indoc::{formatdoc, writedoc};
use sha2::{Digest, Sha256};
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::{self, Command};
use tempdir::TempDir;

fn write_latex(out: &mut impl io::Write, preamble: &str, latex: &str) -> Result<(), io::Error> {
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

pub fn latex_to_svg(preamble: &str, latex: &str) -> Result<String, LatexToSvgError> {
    let tmp_dir = TempDir::new("latex-to-html")?;

    let latex_file_path = tmp_dir.path().join("doc.tex");
    let pdf_file_path = tmp_dir.path().join("doc.pdf");
    let pdf_crop_file_path = tmp_dir.path().join("doc-crop.pdf");
    let svg_file_path = tmp_dir.path().join("doc.svg");

    let mut latex_file = File::create(&latex_file_path).map_err(LatexToSvgError::Io)?;
    write_latex(&mut latex_file, preamble, latex)?;

    let mut pdf_latex_cmd = Command::new("pdflatex");
    pdf_latex_cmd.current_dir(tmp_dir.path());
    pdf_latex_cmd.arg("-interaction=nonstopmode");
    pdf_latex_cmd.arg(&latex_file_path);
    let pdf_latex_output = pdf_latex_cmd.output()?;
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

#[test]
fn test_latex_to_svg() {
    let preamble = indoc::indoc! {r#"
        \usepackage[utf8]{inputenc}
        \usepackage[english]{babel}
        \usepackage{amsfonts}
        \usepackage{amsmath}
        \usepackage{amsthm}
        \usepackage{amssymb}
        \usepackage{dsfont}
        \usepackage{url}
        \usepackage{hyperref}
        \usepackage{tikz-cd}
        \usepackage{mathtools}
        
        \mathtoolsset{showonlyrefs}
    "#};

    let math = indoc::indoc! {r#"
        \begin{equation}
            \begin{tikzcd}
              A \arrow[r] \arrow[d] & X \\
              B \arrow[ur, dashed]
            \end{tikzcd}
        \end{equation}
    "#};

    latex_to_svg(preamble, math).unwrap();
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
    preamble: &str,
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

pub struct MathDigest(pub [u8; 32]);

impl Display for MathDigest {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        write!(out, "{}", hex::encode(self.0))?;
        Ok(())
    }
}

pub fn hash_math(preamble: &str, math: &Math) -> MathDigest {
    let mut hasher = Sha256::new();

    hasher.update(preamble.as_bytes());

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
    preamble: &str,
    math: impl Iterator<Item = &'b Math<'b>>,
) -> Result<(), (&'b Math<'b>, LatexToSvgError)> {
    let out_dir = out_dir.join(SVG_OUT_DIR);
    fs::create_dir_all(&out_dir).unwrap();

    let geometry_path = out_dir.join("geometry.css");
    let mut geometry_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(geometry_path)
        .unwrap();

    for math in math {
        let digest = hash_math(preamble, &math);

        let svg_filename = format!("{digest}.svg");
        let svg_path = out_dir.join(&svg_filename);
        if svg_path.exists() {
            continue;
        }

        let (
            svg,
            SvgInfo {
                width_em,
                height_em,
                baseline_em,
            },
        ) = math_to_svg(preamble, math).map_err(|err| (math, err))?;

        let svg_path_tmp = svg_path.with_file_name(format!("{svg_filename}.tmp"));
        fs::write(&svg_path_tmp, &String::from(&svg)).unwrap();

        let top_em = match baseline_em {
            None => 0.0,
            Some(baseline_em) => height_em - baseline_em,
        };
        writedoc! {geometry_file, r#"
            img[src$="{digest}.svg"] {{
                width: {width_em}em;
                height: {height_em}em;
                top: {top_em}em;
            }}
        "#}
        .unwrap();
        geometry_file.sync_data().unwrap();

        fs::rename(svg_path_tmp, svg_path).unwrap();
    }

    Ok(())
}

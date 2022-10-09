use crate::ast::*;
use crate::util::*;
use indoc::{formatdoc, writedoc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use tempdir::TempDir;

fn write_latex(out: &mut impl io::Write, preamble: &str, latex: &str) -> Result<(), io::Error> {
    writedoc! {out, r#"
        \documentclass{{minimal}}
        {preamble}
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
        return Err(LatexToSvgError::PdfCrop(pdf2svg_output));
    }

    let svg = std::fs::read_to_string(&svg_file_path).unwrap();
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

pub struct InlineMathSvg {
    pub svg: String,
    pub baseline_em: f64,
    pub height_em: f64,
}

pub fn svg_height_to_em(
    mut svg: minidom::Element,
) -> Result<(minidom::Element, f64), LatexToSvgError> {
    let bad_svg = || LatexToSvgError::BadSvg;

    let height_attr = svg.attr("height").ok_or(bad_svg())?;
    let height_pt: f64 = height_attr
        .strip_suffix("pt")
        .ok_or(bad_svg())?
        .parse()
        .map_err(|_| bad_svg())?;

    let height_em = height_pt / 10.0;
    svg.set_attr("height", format!("{height_em}em"));
    svg.set_attr("width", format!("100%"));

    Ok((svg, height_em))
}

pub fn inline_math_to_svg(preamble: &str, math: &str) -> Result<InlineMathSvg, LatexToSvgError> {
    let wrapped_math = formatdoc! {r#"
        $\makebox[0pt][l]{{\rule{{1pt}}{{1pt}}}}{math}$
    "#};

    let svg = latex_to_svg(preamble, &wrapped_math)?;

    let bad_svg = || LatexToSvgError::BadSvg;
    let svg_el: minidom::Element = svg.parse().map_err(|_| bad_svg())?;
    let (mut svg_el, height_em) = svg_height_to_em(svg_el)?;

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
    Ok(InlineMathSvg {
        svg: String::from(&svg_el),
        baseline_em,
        height_em,
    })
}

#[test]
fn test_inline_math_to_svg() {
    inline_math_to_svg("", "5 + 3 + N").unwrap();
}

pub struct DisplayMathSvg(pub String);

pub fn display_math_to_svg(preamble: &str, math: &str) -> Result<DisplayMathSvg, LatexToSvgError> {
    let wrapped_math = formatdoc! {r#"
        \begin{{equation}}
            {math}
        \end{{equation}}
    "#};
    let svg = latex_to_svg(preamble, &wrapped_math)?;

    let bad_svg = || LatexToSvgError::BadSvg;

    let svg_el: minidom::Element = svg.parse().map_err(|_| bad_svg())?;
    let (svg_el, _) = svg_height_to_em(svg_el)?;

    Ok(DisplayMathSvg(String::from(&svg_el)))
}

pub fn mathpar_math_to_svg(preamble: &str, math: &str) -> Result<DisplayMathSvg, LatexToSvgError> {
    let wrapped_math = formatdoc! {r#"
        \begin{{mathpar}}
            {math}
        \end{{mathpar}}
    "#};
    let svg = latex_to_svg(preamble, &wrapped_math)?;

    let bad_svg = || LatexToSvgError::BadSvg;

    let svg_el: minidom::Element = svg.parse().map_err(|_| bad_svg())?;
    let (svg_el, _) = svg_height_to_em(svg_el)?;

    Ok(DisplayMathSvg(String::from(&svg_el)))
}

type MathDigest = [u8; 32];

fn hash_math(preamble: &str, math: &Math) -> MathDigest {
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

    hasher.finalize().as_slice().try_into().unwrap()
}

pub fn display_svg_math_path<'a>(preamble: &'a str, math: &'a Math<'a>) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        let hash: String = hex::encode(hash_math(preamble, math));
        write!(out, "img-math/{hash}.svg").unwrap();
        Ok(())
    })
}

pub struct MathSvgInfo {
    pub path: PathBuf,
    pub y_em_offset: Option<f64>,
}

pub fn create_math_svg_files<'a, 'b>(
    root: &'a Path,
    preamble: &str,
    math: impl Iterator<Item = &'b Math<'b>>,
) -> HashMap<&'b Math<'b>, MathSvgInfo> {
    fs::create_dir_all(root.join("img-math")).unwrap();

    let mut result = HashMap::new();

    math.for_each(|math| {
        if result.contains_key(&math) {
            return;
        }

        let mut info = MathSvgInfo {
            y_em_offset: None,
            path: PathBuf::from(format!("{}", display_svg_math_path(preamble, math))),
        };
        let path = root.join(&info.path);
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }

        use Math::*;
        let svg = match math {
            Inline(src) => {
                let InlineMathSvg {
                    svg,
                    height_em,
                    baseline_em,
                } = inline_math_to_svg(preamble, src).unwrap();
                info.y_em_offset = Some(height_em - baseline_em);
                svg
            }
            Display { source, label: _ } => {
                let DisplayMathSvg(svg) = display_math_to_svg(preamble, source).unwrap();
                svg
            }
            Mathpar { source, label: _ } => {
                let DisplayMathSvg(svg) = mathpar_math_to_svg(preamble, source).unwrap();
                svg
            }
        };

        fs::write(&path, svg).unwrap();

        result.insert(math, info);
    });

    result
}

pub fn display_svg_style<'a>(infos: &'a HashMap<&'a Math<'a>, MathSvgInfo>) -> impl 'a + Display {
    DisplayFn(|out: &mut Formatter| {
        for info in infos.values() {
            if let Some(y_em_offset) = info.y_em_offset {
                let path = info.path.display();
                writedoc! {out, r#"
                    img[src="{path}"] {{
                    top: {y_em_offset}em;
                    }}
                "#}?;
            }
        }
        Ok(())
    })
}

#[test]
fn test_display_math_to_svg() {
    display_math_to_svg("", "5 + 3 + N").unwrap();
    mathpar_math_to_svg("\\usepackage{mathpartir}", "5 + 3 + N").unwrap();
}

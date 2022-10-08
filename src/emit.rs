use crate::ast::*;
use crate::math_svg::*;
use indoc::writedoc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result, Write};
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::write;

struct DisplayFn<F: Fn(&mut Formatter) -> Result>(F);

impl<F: Fn(&mut Formatter) -> Result> Display for DisplayFn<F> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0(f)
    }
}

type MathDigest = [u8; 32];

fn hash_math(preamble: &str, math: Math) -> MathDigest {
    let mut hasher = Sha256::new();

    hasher.update(preamble.as_bytes());

    use Math::*;
    match math {
        Inline(src) => {
            hasher.update(&[0]);
            hasher.update(src);
        }
        Display(src) => {
            hasher.update(&[1]);
            hasher.update(src);
        }
        Mathpar(src) => {
            hasher.update(&[2]);
            hasher.update(src);
        }
    }

    hasher.finalize().as_slice().try_into().unwrap()
}

fn display_svg_math_path<'a>(preamble: &'a str, math: Math<'a>) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        let hash: String = hex::encode(hash_math(preamble, math));
        write!(out, "img-math/{hash}.svg").unwrap();
        Ok(())
    })
}

pub struct MathSvgInfo {
    path: PathBuf,
    y_em_offset: Option<f64>,
}

fn create_math_svg_files<'a>(root: &'a Path, doc: &'a Document) -> HashMap<Math<'a>, MathSvgInfo> {
    fs::create_dir_all(root.join("img-math")).unwrap();

    let mut result = HashMap::new();

    doc.for_each_math(|math| {
        if result.contains_key(&math) {
            return;
        }

        let mut info = MathSvgInfo {
            y_em_offset: None,
            path: PathBuf::from(format!("{}", display_svg_math_path(doc.preamble, math))),
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
                } = inline_math_to_svg(doc.preamble, src).unwrap();
                info.y_em_offset = Some(height_em - baseline_em);
                svg
            }
            Display(src) => {
                let DisplayMathSvg(svg) = display_math_to_svg(doc.preamble, src).unwrap();
                svg
            }
            Mathpar(src) => {
                let DisplayMathSvg(svg) = mathpar_math_to_svg(doc.preamble, src).unwrap();
                svg
            }
        };

        fs::write(&path, svg).unwrap();

        result.insert(math, info);
    });

    result
}

fn display_math<'a>(preamble: &'a str, math: Math<'a>) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        let path = display_svg_math_path(preamble, math);
        use Math::*;
        let class = match math {
            Inline(_) => "inline-math",
            Display(_) | Mathpar(_) => "display-math",
        };
        write!(out, r#"<img src="{path}" class="{class}">"#)?;
        Ok(())
    })
}

fn display_paragraph_part<'a>(preamble: &'a str, part: &'a ParagraphPart) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        use ParagraphPart::*;
        match part {
            InlineWhitespace(ws) => out.write_str(ws)?,
            TextToken(tok) => out.write_str(tok)?,
            Math(math) => {
                write!(out, "{}", display_math(preamble, *math))?;
            }
            Ref(_) => {}
            Eqref(_) => {}
            Emph(child_paragraph) => {
                let child_displ = display_paragraph(preamble, child_paragraph);
                write!(out, "<emph>{child_displ}</emph>")?;
            }
            Comment(_) => {}
            Label(_) => {}
            Qed => {}
            Itemize(items) => {
                write!(out, "<ul>\n")?;
                for item in items {
                    write!(out, "<li>\n")?;
                    for paragraph in item {
                        display_paragraph(preamble, paragraph).fmt(out)?;
                    }
                    write!(out, "</li>\n")?;
                }
                write!(out, "</ul>\n")?;
            }
            Enumerate(items) => {
                write!(out, "<ol>\n")?;
                for item in items {
                    write!(out, "<li>\n")?;
                    for paragraph in item {
                        display_paragraph(preamble, paragraph).fmt(out)?;
                    }
                    write!(out, "</li>\n")?;
                }
                write!(out, "</ol>\n")?;
            }
            Todo => (),
        }
        Ok(())
    })
}

fn display_paragraph<'a>(preamble: &'a str, paragraph: &'a Paragraph) -> impl 'a + Display {
    DisplayFn(|out: &mut Formatter| {
        for part in paragraph.iter() {
            write!(out, "{}", display_paragraph_part(preamble, part))?;
        }
        Ok(())
    })
}

pub fn display_head(title: impl Display) -> impl Display {
    DisplayFn(move |out: &mut Formatter| {
        writedoc! {out, r#"
              <head>
              <meta charset="utf-8">
              <title>{title}</title>
              <link rel="stylesheet" type="text/css" href="https://cdn.rawgit.com/dreampulse/computer-modern-web-font/master/fonts.css">
              <link rel="stylesheet" type="text/css" href="img-math/style.css">
              <style>
              body {{
              font-family: "Computer Modern Serif", serif;
              }}
              </style>
              </head>
        "#}?;
        Ok(())
    })
}

pub fn write_index(out: &mut impl Write, doc: &Document) -> Result {
    let head = display_head("Render experiment");
    writedoc! {out, r#"
        <!DOCTYPE html>
        <html lang="en">
        {head}
        <body>
    "#}?;

    let preamble = &doc.preamble;
    let config = &doc.config;

    for part in doc.parts.iter() {
        use DocumentPart::*;
        match part {
            FreeParagraph(p) => {
                write!(out, "<p>\n")?;
                write!(out, "{}", display_paragraph(preamble, p))?;
                write!(out, "</p>\n")?;
            }
            Title(_) => (),
            Author(_) => (),
            Date() => (),
            Maketitle() => (),
            Section(p) => {
                write!(out, "<h2>\n")?;
                write!(out, "{}", display_paragraph(preamble, p))?;
                write!(out, "</h2>\n")?;
            }
            Subsection(p) => {
                write!(out, "<h3>\n")?;
                write!(out, "{}", display_paragraph(preamble, p))?;
                write!(out, "</h3>\n")?;
            }
            Abstract(ps) => {
                write!(out, "<h2>Abstract</h2>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write!(out, "{}", display_paragraph(preamble, p))?;
                    write!(out, "<p>\n")?;
                }
            }
            TheoremLike(theorem_like) => {
                let theorem_like_config = config
                    .theorem_like_configs
                    .iter()
                    .find(|config| config.tag == theorem_like.tag)
                    .unwrap();
                let name = display_paragraph(preamble, &theorem_like_config.name);
                writedoc! {out, "
                    <div>
                    <h4>
                    {name}
                    </h4>
                "}?;
                for parag in theorem_like.content.iter() {
                    let parag_displ = display_paragraph(preamble, parag);
                    writedoc! {out, "
                        {parag_displ}
                    "}?;
                }
                writedoc! {out, "
                    </div>
                "}?;
            }
            Proof(ps) => {
                write!(out, "<emph>Proof.</emph>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write!(out, "{}", display_paragraph(preamble, p))?;
                    write!(out, "<p>\n")?;
                }
            }
            Label(_) => (),
        }
    }
    writedoc! {out, r#"
        </body>
        </html>
    "#}?;

    Ok(())
}

pub fn display_svg_style<'a>(infos: &'a HashMap<Math<'a>, MathSvgInfo>) -> impl 'a + Display {
    DisplayFn(|out: &mut Formatter| {
        writedoc! {out, r#"
            .inline-math {{
            vertical-align: baseline;
            position: relative;
            }}

            .display-math {{
            display: block;
            margin: auto;
            margin-top: 0.5em;
            margin-bottom: 0.5em;
            }}

        "#}?;

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

pub fn emit(root: &Path, doc: &Document) {
    fs::create_dir_all(root).unwrap();

    let mut index_src = String::new();
    write_index(&mut index_src, &doc).unwrap();

    let index_path = root.join("index.html");
    let mut index_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(index_path)
        .unwrap();
    write!(index_file, "{}", index_src).unwrap();

    let svg_infos = create_math_svg_files(root, doc);

    let svg_style_path = root.join("img-math/style.css");
    let mut svg_style_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(svg_style_path)
        .unwrap();
    write!(svg_style_file, "{}", display_svg_style(&svg_infos)).unwrap();
}

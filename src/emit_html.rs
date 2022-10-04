use crate::ast::*;
use crate::math_svg::*;
use indoc::writedoc;
use std::fmt::{Display, Formatter, Result, Write};
use std::write;

struct DisplayFn<F: Fn(&mut Formatter) -> Result>(F);

impl<F: Fn(&mut Formatter) -> Result> Display for DisplayFn<F> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0(f)
    }
}

fn display_svg_src_attr(svg: &str) -> impl '_ + Display {
    DisplayFn(|out: &mut Formatter| {
        let base64_src = base64::encode(svg.as_bytes());
        write!(out, "data:image/svg+xml;base64,{base64_src}")?;
        Ok(())
    })
}

fn write_paragraph(out: &mut impl Write, preamble: &str, p: &Paragraph) -> Result {
    for part in p.iter() {
        use ParagraphPart::*;
        match part {
            InlineWhitespace(ws) => out.write_str(ws)?,
            TextToken(tok) => out.write_str(tok)?,
            InlineMath(math) => {
                let InlineMathSvg {
                    svg,
                    baseline_pt,
                    height_pt,
                } = inline_math_to_svg(preamble, math).unwrap();
                let img_src = display_svg_src_attr(&svg);
                let top_em = (height_pt - baseline_pt) / 10.0;
                writedoc! {out, "
                    <img class=\"inline-formula\"
                        style=\"vertical-align: baseline; position: relative; top: {top_em}em;\"
                        src=\"{img_src}\">
                "}?;
            }
            DisplayMath(math) => {
                let DisplayMathSvg(svg) = display_math_to_svg(preamble, math).unwrap();
                let img_src = display_svg_src_attr(&svg);
                writedoc! {out, "
                    <img class=\"inline-formula\"
                        style=\"display: block; margin: auto; margin-top: 0.5em; margin-bottom:0.5em\"
                        src=\"{img_src}\">
                "}?;
            }
            Mathpar(_) => {}
            Ref(_) => {}
            Eqref(_) => {}
            Emph(paragraph) => {
                write!(out, "<emph>")?;
                write_paragraph(out, preamble, paragraph)?;
                write!(out, "</emph>\n")?;
            }
            Comment(_) => {}
            Label(_) => {}
            Qed => {}
            Itemize(items) => {
                write!(out, "<ul>\n")?;
                for item in items {
                    write!(out, "<li>\n")?;
                    for paragraph in item {
                        write_paragraph(out, preamble, paragraph)?;
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
                        write_paragraph(out, preamble, paragraph)?;
                    }
                    write!(out, "</li>\n")?;
                }
                write!(out, "</ol>\n")?;
            }
            Todo => (),
        }
    }

    Ok(())
}

pub fn write_document(out: &mut impl Write, doc: &Document) -> Result {
    writedoc! {out, r#"
        <!DOCTYPE html>
        <html lang="en">
          <head>
            <meta charset="utf-8">
            <title>Eqlog render experiment</title>
            <link rel="stylesheet" type="text/css" href="https://cdn.rawgit.com/dreampulse/computer-modern-web-font/master/fonts.css">
            <style>
              body {{
                font-family: "Computer Modern Serif", serif;
              }}
            </style>
          </head>
          <body>
    "#}?;

    let preamble = &doc.preamble;
    for part in doc.parts.iter() {
        use DocumentPart::*;
        match part {
            FreeParagraph(p) => {
                write!(out, "<p>\n")?;
                write_paragraph(out, preamble, p)?;
                write!(out, "</p>\n")?;
            }
            Title(_) => (),
            Author(_) => (),
            Date() => (),
            Maketitle() => (),
            Section(p) => {
                write!(out, "<h2>\n")?;
                write_paragraph(out, preamble, p)?;
                write!(out, "</h2>\n")?;
            }
            Subsection(p) => {
                write!(out, "<h3>\n")?;
                write_paragraph(out, preamble, p)?;
                write!(out, "</h3>\n")?;
            }
            Abstract(ps) => {
                write!(out, "<h2>Abstract</h2>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Proposition(ps) => {
                write!(out, "<h4>Proposition</h4>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Definition(ps) => {
                write!(out, "<h4>Definition</h4>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Lemma(ps) => {
                write!(out, "<h4>Lemma</h4>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Remark(ps) => {
                write!(out, "<h4>Remark</h4>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Corollary(ps) => {
                write!(out, "<h4>Corollary</h4>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Theorem(ps) => {
                write!(out, "<h4>Theorem</h4>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
                    write!(out, "<p>\n")?;
                }
            }
            Proof(ps) => {
                write!(out, "<emph>Proof.</emph>\n")?;
                for p in ps {
                    write!(out, "<p>\n")?;
                    write_paragraph(out, preamble, p)?;
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

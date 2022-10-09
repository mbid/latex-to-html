use crate::ast::*;
use crate::math_svg::*;
use convert_case::{Case, Casing};
use indoc::{indoc, writedoc};
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

fn display_svg_math_path<'a>(preamble: &'a str, math: &'a Math<'a>) -> impl 'a + Display {
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

// fn create_math_svg_files<'a>(root: &'a Path, doc: &'a Document) -> HashMap<Math<'a>, MathSvgInfo> {
fn create_math_svg_files<'a, 'b>(
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

fn display_math<'a>(
    preamble: &'a str,
    math_numbering: &'a HashMap<*const Math<'a>, String>,
    math: &'a Math<'a>,
) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        let path = display_svg_math_path(preamble, math);
        use Math::*;
        match math {
            Inline(_) => {
                write!(out, r#"<img src="{path}" class="inline-math">"#)?;
            }
            Display { source: _, label } | Mathpar { source: _, label } => {
                let id_attr = display_label_id_attr(*label);
                let number = math_numbering.get(&std::ptr::addr_of!(*math));
                writedoc! {out, r#"
                    <div{id_attr} class="display-math-row">
                "#}?;

                if let Some(number) = number {
                    writedoc! {out, r#"
                        <span>{number}</span>
                    "#}?;
                }
                writedoc! {out, r#"
                    <img src="{path}">
                "#}?;
                if let Some(number) = number {
                    writedoc! {out, r#"
                            <span>{number}</span>
                        "#}?;
                }
                writedoc! {out, r#"
                </div>"#}?;
            }
        }
        Ok(())
    })
}

fn assign_math_numberings<'a>(node_lists: &NodeLists<'a>) -> HashMap<*const Math<'a>, String> {
    let mut result: HashMap<*const Math<'a>, String> = HashMap::new();
    let mut current_number = 0;
    for math in node_lists.math.iter().copied() {
        if let Some(label) = math.label() {
            if node_lists.refs.iter().find(|l| **l == label).is_some() {
                current_number += 1;
                result.insert(math, format!("({current_number})"));
            }
        }
    }
    result
}

struct EmitData<'a> {
    preamble: &'a str,
    // The number strings assigned to theorem-like document parts:
    // - TheoremLike
    // - Section
    // - Subsection
    numbering: HashMap<*const DocumentPart<'a>, String>,
    // Number strings assigned to equations.
    math_numbering: HashMap<*const Math<'a>, String>,
    // The text by which references should refer to what they are referencing.
    label_names: HashMap<&'a str, String>,
}

impl<'a> EmitData<'a> {
    fn new(doc: &'a Document<'a>, node_lists: &NodeLists<'a>) -> Self {
        let numbering = assign_numberings(doc);
        let math_numbering = assign_math_numberings(node_lists);
        let label_names = assign_label_names(doc, node_lists, &numbering, &math_numbering);
        EmitData {
            preamble: &doc.preamble,
            numbering,
            math_numbering,
            label_names,
        }
    }
}

fn display_label_id_attr(label_value: Option<&str>) -> impl '_ + Display {
    DisplayFn(move |out: &mut Formatter| {
        let label_value = match label_value {
            None => {
                return Ok(());
            }
            Some(label_value) => display_label_value(label_value),
        };
        write!(out, r#" id="{label_value}""#)?;
        Ok(())
    })
}

fn display_paragraph_part<'a>(
    data: &'a EmitData<'a>,
    part: &'a ParagraphPart,
) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        use ParagraphPart::*;
        match part {
            InlineWhitespace(ws) => {
                let has_newlines = ws.find('\n').is_some();
                if has_newlines {
                    write!(out, "\n")?;
                } else if !ws.is_empty() {
                    write!(out, " ")?;
                }
            }
            TextToken(tok) => out.write_str(tok)?,
            Math(math) => {
                write!(
                    out,
                    "{}",
                    display_math(data.preamble, &data.math_numbering, math)
                )?;
            }
            Ref(value) => {
                let name = match data.label_names.get(value) {
                    None => "???",
                    Some(name) => name.as_str(),
                };
                let value = display_label_value(value);
                write!(out, "<a href=\"#{value}\">{name}</a>")?;
            }
            Emph(child_paragraph) => {
                write!(out, "<emph>")?;
                for part in child_paragraph.iter() {
                    write!(out, "{}", display_paragraph_part(data, part))?;
                }
                write!(out, "</emph>")?;
            }
            Qed => {}
            Itemize(items) => {
                write!(out, "<ul>\n")?;
                for item in items {
                    assert!(item.label.is_none());
                    write!(out, "<li>\n")?;
                    for paragraph in item.content.iter() {
                        display_paragraph(data, paragraph).fmt(out)?;
                    }
                    write!(out, "</li>\n")?;
                }
                write!(out, "</ul>\n")?;
            }
            Enumerate(items) => {
                write!(out, "<ol>\n")?;
                for item in items {
                    let id_attr = display_label_id_attr(item.label);
                    write!(out, "<li{id_attr}>\n")?;
                    for paragraph in item.content.iter() {
                        display_paragraph(data, paragraph).fmt(out)?;
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

fn display_paragraph<'a>(data: &'a EmitData<'a>, paragraph: &'a Paragraph) -> impl 'a + Display {
    DisplayFn(|out: &mut Formatter| {
        writedoc! {out, r#"
            <div class="paragraph">
        "#}?;
        for part in paragraph.iter() {
            write!(out, "{}", display_paragraph_part(data, part))?;
        }
        writedoc! {out, r#"
            </div>
        "#}?;
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
              <link rel="stylesheet" type="text/css" href="style.css">
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

fn display_label_value(label_value: &str) -> impl '_ + Display {
    label_value.replace(":", "-").to_case(Case::Kebab)
}

fn display_theorem_header<'a>(
    data: &'a EmitData,
    name: &'a Paragraph<'a>,
    number: Option<&'a str>,
) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        write!(out, "<h4>")?;
        for part in name.iter() {
            write!(out, "{}", display_paragraph_part(data, part))?;
        }
        if let Some(number) = number {
            write!(out, " {number}")?;
        }
        write!(out, ".\n")?;

        write!(out, "</h4>")?;
        Ok(())
    })
}

fn write_index(out: &mut impl Write, doc: &Document, data: &EmitData) -> Result {
    let head = display_head("Render experiment");
    writedoc! {out, r#"
        <!DOCTYPE html>
        <html lang="en">
        {head}
        <body>
    "#}?;

    let config = &doc.config;

    for part in doc.parts.iter() {
        use DocumentPart::*;
        match part {
            FreeParagraph(p) => {
                write!(out, "{}", display_paragraph(data, p))?;
            }
            Title(_) => (),
            Author(_) => (),
            Date() => (),
            Maketitle() => (),
            Section { name, label } => {
                let label = display_label_id_attr(*label);
                write!(out, "<h2{label}>\n")?;
                let number = data
                    .numbering
                    .get(&std::ptr::addr_of!(*part))
                    .map(|s| s.as_str());
                if let Some(number) = number {
                    write!(out, "{number} ")?;
                }
                for part in name {
                    write!(out, "{}", display_paragraph_part(data, part))?;
                }
                write!(out, "</h2>\n")?;
            }
            Subsection { name, label } => {
                let label = display_label_id_attr(*label);
                write!(out, "<h3{label}>\n")?;
                let number = data
                    .numbering
                    .get(&std::ptr::addr_of!(*part))
                    .map(|s| s.as_str());
                if let Some(number) = number {
                    write!(out, "{number} ")?;
                }
                for part in name {
                    write!(out, "{}", display_paragraph_part(data, part))?;
                }
                write!(out, "</h3>\n")?;
            }
            Abstract(ps) => {
                write!(out, "<h2>Abstract</h2>\n")?;
                for p in ps {
                    write!(out, "{}", display_paragraph(data, p))?;
                }
            }
            TheoremLike {
                tag,
                content,
                label,
            } => {
                let theorem_like_config = config
                    .theorem_like_configs
                    .iter()
                    .find(|config| &config.tag == tag)
                    .unwrap();
                let label = display_label_id_attr(*label);
                let number = data
                    .numbering
                    .get(&std::ptr::addr_of!(*part))
                    .map(|s| s.as_str());
                let header = display_theorem_header(data, &theorem_like_config.name, number);
                writedoc! {out, r#"
                    <div{label} class="theorem-like">
                    <div class="paragraph">
                    {header}
                "#}?;

                let mut content = content.iter();
                if let Some(parag) = content.next() {
                    for part in parag {
                        write!(out, "{}", display_paragraph_part(data, part))?;
                    }
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
                for parag in content {
                    write!(out, "{}", display_paragraph(data, parag))?;
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
            }
            Proof(ps) => {
                writedoc! {out, r#"
                    <div class="proof">
                    <div class="paragraph">
                    <i class="proof">Proof.</i>
                "#}?;
                let mut ps = ps.iter();
                if let Some(parag) = ps.next() {
                    for part in parag {
                        write!(out, "{}", display_paragraph_part(data, part))?;
                    }
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
                for p in ps {
                    write!(out, "{}", display_paragraph(data, p))?;
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
            }
        }
    }
    writedoc! {out, r#"
        </body>
        </html>
    "#}?;

    Ok(())
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

pub fn assign_numberings<'a>(doc: &Document<'a>) -> HashMap<*const DocumentPart<'a>, String> {
    let mut map: HashMap<*const DocumentPart<'a>, String> = HashMap::new();
    let mut current_theorem_like = 0;
    let mut current_section = 0;
    let mut current_subsection = 0;
    for part in doc.parts.iter() {
        match part {
            DocumentPart::TheoremLike { .. } => {
                current_theorem_like += 1;
                map.insert(part, current_theorem_like.to_string());
            }
            DocumentPart::Section { .. } => {
                current_section += 1;
                current_subsection = 0;
                map.insert(part, current_section.to_string());
            }
            DocumentPart::Subsection { .. } => {
                current_subsection += 1;
                map.insert(part, format!("{current_section}.{current_subsection}"));
            }
            _ => (),
        }
    }
    map
}

pub fn assign_label_names<'a>(
    doc: &Document<'a>,
    node_lists: &NodeLists<'a>,
    numbering: &HashMap<*const DocumentPart, String>,
    math_numbering: &HashMap<*const Math, String>,
) -> HashMap<&'a str, String> {
    let mut names = HashMap::new();
    for part in doc.parts.iter() {
        use DocumentPart::*;
        match part {
            TheoremLike { label, .. } | Section { label, .. } | Subsection { label, .. } => {
                if let Some(label) = label {
                    let number = numbering.get(&std::ptr::addr_of!(*part)).unwrap();
                    names.insert(*label, number.clone());
                }
            }
            _ => (),
        }
    }

    for item_list in node_lists.item_lists.iter() {
        for (i, item) in item_list.iter().enumerate() {
            if let Some(label) = item.label {
                names.insert(label, (i + 1).to_string());
            }
        }
    }

    for math in node_lists.math.iter().copied() {
        if let Some(label) = math.label() {
            if let Some(number) = math_numbering.get(&std::ptr::addr_of!(*math)) {
                names.insert(label, number.clone());
            }
        }
    }
    names
}

const STYLE: &'static str = indoc! {"
    h4 {
        display: inline;
    }

    .inline-math {
        vertical-align: baseline;
        position: relative;
    }

    .display-math-row {
        display: flex;
        justify-content: center;
        flex-direction: row;
        align-items: center;
        margin-top: 0.5em;
        margin-bottom: 0.5em;
    }

    .display-math-row > span {
        display: inline-flex;
        flex-direction: row-reverse;
        flex-grow: 1;
    }

    .display-math-row > span:first-child {
        visibility: hidden;
    }"};

pub fn emit(root: &Path, doc: &Document) {
    let node_lists = NodeLists::new(doc);
    let data = EmitData::new(doc, &node_lists);

    fs::create_dir_all(root).unwrap();

    let mut index_src = String::new();
    write_index(&mut index_src, &doc, &data).unwrap();

    let index_path = root.join("index.html");
    let mut index_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(index_path)
        .unwrap();
    write!(index_file, "{}", index_src).unwrap();

    let style_path = root.join("style.css");
    let mut style_path = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(style_path)
        .unwrap();
    write!(style_path, "{STYLE}").unwrap();

    let svg_infos = create_math_svg_files(root, &doc.preamble, node_lists.math.iter().copied());

    let svg_style_path = root.join("img-math/style.css");
    let mut svg_style_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(svg_style_path)
        .unwrap();
    write!(svg_style_file, "{}", display_svg_style(&svg_infos)).unwrap();
}

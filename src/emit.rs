use crate::analysis::*;
use crate::ast::*;
use crate::math_svg::*;
use crate::util::*;
use convert_case::{Case, Casing};
use indoc::{indoc, writedoc};
use itertools::Itertools;
use std::fmt::{Display, Formatter, Result, Write};
use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;
use std::ptr::addr_of;
use std::write;

fn display_math<'a>(analysis: &'a Analysis<'a>, math: &'a Math<'a>) -> impl 'a + Display {
    let src = analysis.math_image_source.get(&addr_of!(*math)).unwrap();
    let number = analysis.math_numbering.get(&addr_of!(*math));
    DisplayFn(move |out: &mut Formatter| {
        use Math::*;
        match math {
            Inline(_) => {
                write!(out, r#"<img src="{src}" class="inline-math">"#)?;
            }
            Display { source: _, label } | Mathpar { source: _, label } => {
                let id_attr = display_label_id_attr(*label);
                writedoc! {out, r#"
                    <div{id_attr} class="display-math-row">
                "#}?;

                if let Some(number) = number {
                    writedoc! {out, r#"
                        <span>{number}</span>
                    "#}?;
                }
                writedoc! {out, r#"
                    <img src="{src}">
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
    analysis: &'a Analysis<'a>,
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
                write!(out, "{}", display_math(analysis, math))?;
            }
            Ref(value) => {
                let name = match analysis.ref_display_text.get(value) {
                    None => "???",
                    Some(name) => name.as_str(),
                };
                let value = display_label_value(value);
                write!(out, "<a href=\"#{value}\">{name}</a>")?;
            }
            Cite { ids, text } => {
                let links = ids.iter().copied().format_with(", ", |id, f| {
                    let display_text = match analysis.cite_display_text.get(id) {
                        None => "???",
                        Some(name) => name.as_str(),
                    };
                    let id = display_cite_value(id);
                    f(&format_args!("<a href=\"#{id}\">{display_text}</a>"))
                });
                write!(out, "[{links}")?;
                if let Some(text) = text {
                    write!(out, ", ")?;
                    for part in text.iter() {
                        write!(out, "{}", display_paragraph_part(analysis, part))?;
                    }
                }
                write!(out, "]")?;
            }
            Emph(child_paragraph) => {
                write!(out, "<em>")?;
                for part in child_paragraph.iter() {
                    write!(out, "{}", display_paragraph_part(analysis, part))?;
                }
                write!(out, "</em>")?;
            }
            Textbf(paragraph) => {
                write!(out, "<strong>")?;
                for part in paragraph.iter() {
                    write!(out, "{}", display_paragraph_part(analysis, part))?;
                }
                write!(out, "</strong>")?;
            }
            Textit(paragraph) => {
                write!(out, "<i>")?;
                for part in paragraph.iter() {
                    write!(out, "{}", display_paragraph_part(analysis, part))?;
                }
                write!(out, "</i>")?;
            }
            Qed => {}
            Itemize(items) => {
                write!(out, "<ul>\n")?;
                for item in items {
                    assert!(item.label.is_none());
                    write!(out, "<li>\n")?;
                    for paragraph in item.content.iter() {
                        display_paragraph(analysis, paragraph).fmt(out)?;
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
                        display_paragraph(analysis, paragraph).fmt(out)?;
                    }
                    write!(out, "</li>\n")?;
                }
                write!(out, "</ol>\n")?;
            }
            Todo => (),
            Footnote(_) => {
                // TODO
            }
        }
        Ok(())
    })
}

fn display_paragraph<'a>(
    analysis: &'a Analysis<'a>,
    paragraph: &'a Paragraph,
) -> impl 'a + Display {
    DisplayFn(|out: &mut Formatter| {
        writedoc! {out, r#"
            <div class="paragraph">
        "#}?;
        for part in paragraph.iter() {
            write!(out, "{}", display_paragraph_part(analysis, part))?;
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
              <meta name="viewport" content="width=device-width, initial-scale=1" />
              <title>{title}</title>
              <link rel="stylesheet" type="text/css" href="https://cdn.rawgit.com/dreampulse/computer-modern-web-font/master/fonts.css">
              <link rel="stylesheet" type="text/css" href="style.css">
              <link rel="stylesheet" type="text/css" href="{SVG_OUT_DIR}/geometry.css">
              </head>
        "#}?;
        Ok(())
    })
}

fn display_label_value(label_value: &str) -> impl '_ + Display {
    label_value.replace(":", "-").to_case(Case::Kebab)
}

fn display_cite_value(label_value: &str) -> impl '_ + Display {
    label_value.replace(":", "-").to_case(Case::Kebab)
}

fn display_theorem_header<'a>(
    analysis: &'a Analysis,
    name: &'a Paragraph<'a>,
    note: Option<&'a Paragraph<'a>>,
    number: Option<&'a str>,
) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        write!(out, "<h4>")?;
        for part in name.iter() {
            write!(out, "{}", display_paragraph_part(analysis, part))?;
        }
        if let Some(number) = number {
            write!(out, " {number}")?;
        }
        if let Some(note) = note {
            // TODO: Should add style so that this span is not bold.
            write!(out, r#" <span class="theorem-note">("#)?;
            for part in note.iter() {
                write!(out, "{}", display_paragraph_part(analysis, part))?;
            }
            write!(out, ")</span>")?;
        }
        write!(out, ".\n")?;

        write!(out, "</h4>")?;
        Ok(())
    })
}

fn display_title<'a>(title: Option<&'a Paragraph<'a>>) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        match title {
            None => (),
            Some(parag) => {
                for part in parag {
                    use ParagraphPart::*;
                    match part {
                        TextToken(tok) => {
                            write!(out, "{tok}")?;
                        }
                        InlineWhitespace(ws) => {
                            if ws.len() > 0 {
                                write!(out, " ")?;
                            }
                        }
                        Math(_)
                        | Ref(_)
                        | Emph(_)
                        | Textbf(_)
                        | Textit(_)
                        | Qed
                        | Enumerate(_)
                        | Itemize(_)
                        | Todo
                        | Cite { .. }
                        | Footnote(_) => {
                            panic!("Invalid node in title");
                        }
                    }
                }
            }
        }
        Ok(())
    })
}

fn display_bib_person<'a>(person: &'a BibPerson<'a>) -> impl 'a + Display {
    DisplayFn(move |out: &mut Formatter| {
        for first_name in person.first_names.iter() {
            use FirstName::*;
            match first_name {
                Full(name) => {
                    write!(out, "{name} ")?;
                }
                Abbreviation(abbr) => {
                    write!(out, "{abbr}. ")?;
                }
            }
        }
        let last_name = person.last_name;
        write!(out, "{last_name}")?;
        Ok(())
    })
}

fn display_bib_entry<'a>(entry: &'a BibEntry<'a>) -> impl 'a + Display {
    let title = entry.title;
    let authors = &entry.authors;

    let id_attr_value = display_cite_value(entry.tag);

    DisplayFn(move |out: &mut Formatter| {
        writedoc! {out, r#"
            <li id="{id_attr_value}">
        "#}?;
        match authors.as_deref() {
            None | Some([]) => (),
            Some([author]) => {
                write!(out, " {}.", display_bib_person(author))?;
            }
            Some([init @ .., before_last, last]) => {
                for author in init {
                    write!(out, " {},", display_bib_person(author))?;
                }
                write!(out, " {}", display_bib_person(before_last))?;
                write!(out, " and {}.", display_bib_person(last))?;
            }
        };
        if let Some(title) = title {
            write!(out, " {title}.")?;
        }

        // TODO: Only on of journal, booktitle or series should be present.
        if let Some(journal) = entry.journal {
            write!(out, " {journal}")?;
        }
        if let Some(booktitle) = entry.booktitle {
            write!(out, " {booktitle}")?;
        }
        if let Some(series) = entry.series {
            write!(out, " {series}")?;
        }

        let has_volume_or_number = match (entry.volume, entry.number) {
            (Some(volume), Some(number)) => {
                write!(out, ", {volume}({number})")?;
                true
            }
            (Some(volume), None) => {
                write!(out, ", {volume}")?;
                true
            }
            (None, Some(number)) => {
                write!(out, ", ({number})")?;
                true
            }
            (None, None) => false,
        };

        if let Some(BibPages { first, last }) = entry.pages {
            if has_volume_or_number {
                write!(out, ":")?;
            } else {
                if last.is_some() {
                    write!(out, ", pages ")?;
                } else {
                    write!(out, ", page ")?;
                }
            }
            write!(out, "{first}")?;
            if let Some(last) = last {
                write!(out, "–{last}")?;
            }
        }

        match (has_volume_or_number || entry.pages.is_some(), entry.year) {
            (true, Some(year)) => {
                write!(out, ", {year}.")?;
            }
            (true, None) => {
                write!(out, ".")?;
            }
            (false, Some(year)) => {
                if entry.journal.is_some() || entry.booktitle.is_some() || entry.series.is_some() {
                    write!(out, ", {year}.")?;
                } else {
                    write!(out, " {year}.")?;
                }
            }
            (false, None) => (),
        };

        writedoc! {out, r#"</li>"#}?;
        Ok(())
    })
}

fn write_index(out: &mut impl Write, doc: &Document, analysis: &Analysis) -> Result {
    let title: Option<&Paragraph> = doc.parts.iter().find_map(|part| {
        if let DocumentPart::Title(title) = part {
            Some(title)
        } else {
            None
        }
    });

    let head = display_head(display_title(title));
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
                write!(out, "{}", display_paragraph(analysis, p))?;
            }
            Title(_) => (),
            Author(_) => (),
            Date() => (),
            Maketitle() => {
                if title.is_some() {
                    let title = display_title(title);
                    writedoc! {out, r#"
                        <h1>{title}</h1>
                    "#}?;
                }
            }
            Section { name, label } => {
                let label = display_label_id_attr(*label);
                write!(out, "<h2{label}>\n")?;
                let number = analysis
                    .doc_part_numbering
                    .get(&std::ptr::addr_of!(*part))
                    .map(|s| s.as_str());
                if let Some(number) = number {
                    write!(out, "{number} ")?;
                }
                for part in name {
                    write!(out, "{}", display_paragraph_part(analysis, part))?;
                }
                write!(out, "</h2>\n")?;
            }
            Subsection { name, label } => {
                let label = display_label_id_attr(*label);
                write!(out, "<h3{label}>\n")?;
                let number = analysis
                    .doc_part_numbering
                    .get(&std::ptr::addr_of!(*part))
                    .map(|s| s.as_str());
                if let Some(number) = number {
                    write!(out, "{number} ")?;
                }
                for part in name {
                    write!(out, "{}", display_paragraph_part(analysis, part))?;
                }
                write!(out, "</h3>\n")?;
            }
            Abstract(ps) => {
                write!(out, "<h2>Abstract</h2>\n")?;
                for p in ps {
                    write!(out, "{}", display_paragraph(analysis, p))?;
                }
            }
            TheoremLike {
                tag,
                note,
                content,
                label,
            } => {
                let theorem_like_config = config
                    .theorem_like_configs
                    .iter()
                    .find(|config| &config.tag == tag)
                    .unwrap();
                let label = display_label_id_attr(*label);
                let number = analysis
                    .doc_part_numbering
                    .get(&std::ptr::addr_of!(*part))
                    .map(|s| s.as_str());
                let header = display_theorem_header(
                    analysis,
                    &theorem_like_config.name,
                    note.as_ref(),
                    number,
                );
                writedoc! {out, r#"
                    <div{label} class="theorem-like">
                    <div class="paragraph">
                    {header}
                "#}?;

                let mut content = content.iter();
                if let Some(parag) = content.next() {
                    for part in parag {
                        write!(out, "{}", display_paragraph_part(analysis, part))?;
                    }
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
                for parag in content {
                    write!(out, "{}", display_paragraph(analysis, parag))?;
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
                        write!(out, "{}", display_paragraph_part(analysis, part))?;
                    }
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
                for p in ps {
                    write!(out, "{}", display_paragraph(analysis, p))?;
                }
                writedoc! {out, r#"
                    </div>
                "#}?;
            }
            Bibliography => {
                writedoc! {out, r#"
                    <h2>Bibliography</h2>
                    <ol class="bibliography">
                "#}?;
                for entry in analysis.bib_entries.iter().copied() {
                    let entry = display_bib_entry(entry);
                    writedoc! {out, r#"
                        {entry}
                    "#}?;
                }
                writedoc! {out, r#"
                    </ol>
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

const STYLE: &'static str = indoc! {r#"
    html {
        padding: 0.5em;
    }
    body {
        font-family: "Computer Modern Serif", serif;
        max-width: 600px;
        margin: auto;
    }

    h4 {
        display: inline;
    }

    .theorem-like {
        margin-top: 0.5em;
        margin-bottom: 0.5em;
    }

    .proof {
        margin-top: 0.5em;
    }

    .inline-math {
        vertical-align: baseline;
        position: relative;
    }

    .display-math-row {
        display: flex;
        flex-direction: row;
        margin-top: 0.5em;
        margin-bottom: 0.5em;
        overflow: auto;
    }

    .display-math-row > img {
        margin: auto;
    }

    .display-math-row > span {
        margin: auto 0;
        display: inline-flex;
        flex-direction: row-reverse;
        padding-left: 1em;
    }

    .display-math-row > span:first-child {
        visibility: hidden;
    }

    .bibliography {
      counter-reset: list;
    }

    .bibliography > li {
      counter-increment: list;
    }

    .bibliography > li::marker {
      content: "["counter(list)"] ";
    }"#};

pub fn emit(root: &Path, doc: &Document, analysis: &Analysis) {
    fs::create_dir_all(root).unwrap();

    let mut index_src = String::new();
    write_index(&mut index_src, &doc, &analysis).unwrap();

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
}

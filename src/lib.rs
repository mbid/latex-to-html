mod analysis;
mod ast;
mod display_source;
mod emit;
mod math_svg;
mod parse;
mod util;

use crate::analysis::Analysis;
use crate::ast::*;
use crate::display_source::*;
use crate::emit::emit;
use crate::math_svg::*;
use crate::parse::{bib, document};
use indoc::{eprintdoc, formatdoc};
use nom::combinator::complete;
use nom::Offset;
use std::path::Path;
use std::process;
use std::str::from_utf8;

fn read_file(file_path: &Path) -> String {
    match std::fs::read_to_string(file_path) {
        Ok(src) => src,
        Err(_) => {
            let file_path = file_path.display();
            eprintdoc! {r#"
                Error: Could not read file "{file_path}"
            "#};
            process::exit(1);
        }
    }
}

fn parse_source<'a, O>(
    parser: impl FnMut(&'a str) -> parse::Result<'a, O>,
    source: &'a str,
    source_path: &'a Path,
) -> O {
    match complete(parser)(source) {
        Ok((_, o)) => o,
        Err(nom::Err::Incomplete(_)) => panic!(),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            //let location = remaining_begin_location(e.input, source);
            let location = Location(source.offset(e.input), source.offset(e.input) + 1);
            let location_display = SourceDisplay {
                source,
                location,
                source_path: Some(source_path),
                underlined: true,
            };
            eprintdoc! {"
                Error: Unexpected token
                {location_display}
            "};
            process::exit(1);
        }
    }
}

pub fn latex_to_html(tex_path: &Path, bib_path: &Path, out_path: &Path) {
    let tex_src = read_file(tex_path);
    let doc = parse_source(document, tex_src.as_str(), tex_path);

    let bib_src = read_file(bib_path);
    let bib_entries = parse_source(bib, bib_src.as_str(), bib_path);

    // Generate lists of nodes and analyze the bib/latex asts.
    let node_lists = NodeLists::new(&doc);
    let analysis = Analysis::new(&doc, &bib_entries, &node_lists);

    emit(&out_path, &doc, &analysis);
    use LatexToSvgError::*;
    match emit_math_svg_files(&out_path, &doc.preamble, node_lists.math.iter().copied()) {
        Ok(()) => (),
        Err((math, error)) => {
            use Math::*;
            let math_source = match math {
                Inline(src) => src,
                Display { source, .. } | Mathpar { source, .. } => source,
            };
            let location_begin = tex_src.offset(math_source);
            let location = Location(location_begin, location_begin + math_source.len());
            debug_assert_eq!(&&tex_src[location.0..location.1], math_source);

            let location_display = SourceDisplay {
                source: tex_src.as_str(),
                location,
                source_path: Some(tex_path),
                underlined: match math {
                    Inline(_) => true,
                    Display { .. } | Mathpar { .. } => false,
                },
            };

            let error_text = match error {
                PdfLatex(output) => from_utf8(&output.stdout).unwrap().to_string(),
                err => formatdoc! {"
                    Internal error while generating svg:
                    {err:?}
                "},
            };
            eprintdoc! {"
                Error: Math formula is invalid
                {location_display}

                {error_text}
            "};
            process::exit(1);
        }
    };
}

#[test]
fn eqlog_example() {
    latex_to_html(
        Path::new("example.tex"),
        Path::new("example.bib"),
        Path::new("out/example"),
    );
}

#[test]
fn lcc_model_example() {
    latex_to_html(
        Path::new("lcc-model.tex"),
        Path::new("lcc-model.bib"),
        Path::new("out/lcc-model"),
    );
}

#[test]
fn stack_project_schemes_example() {
    latex_to_html(
        Path::new("schemes.tex"),
        Path::new("lcc-model.bib"),
        Path::new("out/stacks-project/schemes"),
    );
}

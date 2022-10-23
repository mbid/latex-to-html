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
use indoc::eprintdoc;
use nom::combinator::complete;
use nom::Offset;
use std::iter::repeat;
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

pub fn print_latex_to_svg_error(
    tex_path: &Path,
    tex_src: &str,
    preamble: &[&str],
    math: &Math,
    error: &LatexToSvgError,
) {
    // First obtain the output from just compiling a dummy formuala with the preamble. This way,
    // we can either diagnose problems with the preamble (if there are some) or remove irrelevant
    // parts from the output for compiling the formula at hand.
    let default_output = match diagnose_preamble(preamble).unwrap() {
        PreambleDiagnosis::Ok(output) => output,
        PreambleDiagnosis::OffendingLines(output, lines) => {
            let location = match lines {
                [] => Location(0, 1),
                [line] => {
                    let begin = tex_src.offset(line);
                    let end = begin + line.len();
                    Location(begin, end)
                }
                [first_line, .., last_line] => {
                    let begin = tex_src.offset(first_line);
                    let end = tex_src.offset(last_line) + last_line.len();
                    Location(begin, end)
                }
            };

            let location_display = SourceDisplay {
                source: tex_src,
                location,
                source_path: Some(tex_path),
                underlined: false,
            };

            let stdout = from_utf8(&output.stdout).unwrap();

            eprintdoc! {r#"
                Error: Preamble is invalid
                {location_display}

                Note: Your preamble must be compatible with the "minimal" documentclass.
                      Try adding the line

                        % LATEX_TO_HTML_IGNORE
                         
                      to make latex-to-html ignore the next line.

                ================================================================================
                {stdout}
            "#};
            return;
        }
    };

    use LatexToSvgError::*;
    let pdf_latex_output = match error {
        PdfLatex(output) => output,
        err => {
            eprintdoc! {"
                Internal error while generating svg:
                {err:?}
            "};
            return;
        }
    };

    let stdout = from_utf8(&pdf_latex_output.stdout).unwrap();

    use Math::*;
    let math_source = match math {
        Inline(src) => src,
        Display { source, .. } | Mathpar { source, .. } => source,
    };
    let location_begin = tex_src.offset(math_source);
    let location = Location(location_begin, location_begin + math_source.len());
    debug_assert_eq!(&&tex_src[location.0..location.1], math_source);

    let location_display = SourceDisplay {
        source: tex_src,
        location,
        source_path: Some(tex_path),
        underlined: match math {
            Inline(_) => true,
            Display { .. } | Mathpar { .. } => false,
        },
    };

    eprintdoc! {r#"
        Error: Math formula is invalid
        {location_display}

        ================================================================================
    "#};

    let default_stdout = from_utf8(&default_output.stdout).unwrap();

    // Only display those lines that do not appear in the default output.
    let relevant_lines = stdout
        .lines()
        .zip(default_stdout.lines().map(Some).chain(repeat(None)))
        .filter(|(line, default_line)| match default_line {
            None => true,
            Some(default_line) => line != default_line,
        })
        .map(|(line, _)| line);

    for line in relevant_lines {
        eprintln!("{}", line);
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
    if let Err((math, err)) = emit_math_svg_files(&out_path, &doc.preamble, &node_lists.math) {
        print_latex_to_svg_error(
            tex_path,
            tex_src.as_str(),
            doc.preamble.as_slice(),
            math,
            &err,
        );
        process::exit(1);
    }
}

#[test]
fn example() {
    latex_to_html(
        Path::new("example.tex"),
        Path::new("example.bib"),
        Path::new("out/example"),
    );
}

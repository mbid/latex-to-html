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
    preamble: &str,
    math: &Math,
    error: &LatexToSvgError,
) {
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

    let default_output = default_pdf_latex_output(preamble).unwrap();
    if !default_output.status.success() {
        eprintdoc! {r#"
            Error: Preamble is invalid
            Note: Your preamble must be compatible with the "minimal" documentclass.

            {stdout}
        "#};
        return;
    }

    eprintdoc! {r#"
        Error: Math formula is invalid
        {location_display}

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
            doc.preamble.as_str(),
            math,
            &err,
        );
        process::exit(1);
    }
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

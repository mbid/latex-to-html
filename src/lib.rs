pub mod analysis;
pub mod ast;
pub mod emit;
pub mod math_svg;
pub mod parse;
pub mod util;

#[test]
fn parse_eqlog_paper() {
    use crate::parse::document;
    let src = std::fs::read_to_string("example.tex").unwrap();
    let (i, _) = document(src.as_str()).unwrap();
    assert!(i.is_empty());
}

#[test]
fn example() {
    use crate::analysis::Analysis;
    use crate::ast::NodeLists;
    use crate::emit::emit;
    use crate::math_svg::emit_math_svg_files;
    use crate::parse::{bib, document};

    // Parse the bibliography file.
    let bib_src = std::fs::read_to_string("example.bib").unwrap();
    let (i, bib_entries) = bib(bib_src.as_str()).unwrap();
    assert!(i.is_empty());

    // Parse the latex file.
    let src = std::fs::read_to_string("example.tex").unwrap();
    let (i, doc) = document(src.as_str()).unwrap();
    assert!(i.is_empty());

    // Generate lists of nodes and analyze the bib/latex asts.
    let node_lists = NodeLists::new(&doc);
    let analysis = Analysis::new(&doc, &bib_entries, &node_lists);

    let out_path = std::path::Path::new("out");
    emit(&out_path, &doc, &analysis);
    emit_math_svg_files(&out_path, &doc.preamble, node_lists.math.iter().copied());
}

#[test]
fn lcc_model_example() {
    use crate::analysis::Analysis;
    use crate::ast::NodeLists;
    use crate::emit::emit;
    use crate::math_svg::emit_math_svg_files;
    use crate::parse::{bib, document};

    // Parse the bibliography file.
    let bib_src = std::fs::read_to_string("lcc-model.bib").unwrap();
    let (i, bib_entries) = bib(bib_src.as_str()).unwrap();
    assert!(i.is_empty(), "{}", i);

    // Parse the latex file.
    let src = std::fs::read_to_string("lcc-model.tex").unwrap();
    let (i, doc) = document(src.as_str()).unwrap();
    assert!(i.is_empty());

    // Generate lists of nodes and analyze the bib/latex asts.
    let node_lists = NodeLists::new(&doc);
    let analysis = Analysis::new(&doc, &bib_entries, &node_lists);

    let out_path = std::path::Path::new("out");
    emit(&out_path, &doc, &analysis);
    emit_math_svg_files(&out_path, &doc.preamble, node_lists.math.iter().copied());
}

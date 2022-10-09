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
fn doit() {
    use crate::emit::emit;
    use crate::parse::document;

    let src = std::fs::read_to_string("example.tex").unwrap();
    let (i, doc) = document(src.as_str()).unwrap();
    assert!(i.is_empty());

    emit(std::path::Path::new("out"), &doc);
}

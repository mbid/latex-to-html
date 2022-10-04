pub mod ast;
pub mod emit_html;
pub mod math_svg;
pub mod parse;

#[test]
fn parse_eqlog_paper() {
    use crate::parse::document;
    let src = std::fs::read_to_string("example.tex").unwrap();
    let (i, _) = document(src.as_str()).unwrap();
    assert!(i.is_empty());
}

#[test]
fn doit() {
    use crate::emit_html::write_document;
    use crate::parse::document;

    let src = std::fs::read_to_string("example.tex").unwrap();
    let (i, doc) = document(src.as_str()).unwrap();
    assert!(i.is_empty());

    let mut html = String::new();
    write_document(&mut html, &doc).unwrap();
    std::fs::write("/tmp/example.html", &html).unwrap();
}

pub mod parse;

#[test]
fn parse_eqlog_paper() {
    use crate::parse::{document, Document};
    let src = std::fs::read_to_string("example.tex").unwrap();
    let (i, Document { preamble, parts }) = document(src.as_str()).unwrap();
    assert!(i.is_empty());

    indoc::printdoc! {"
        Preamble:
        ------------------------------------
        {preamble}
        ------------------------------------
    "};
    indoc::printdoc! {"
        Parts:
        ------------------------------------
    "};
    for part in parts.iter() {
        indoc::printdoc! {"
            {part:?}
            ------------------------------------
        "};
    }
}

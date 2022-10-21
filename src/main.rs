use latex_to_html::latex_to_html;
use std::env::args;
use std::path::PathBuf;
use std::process;

fn main() {
    if args().len() != 4 {
        eprintln!("Usage: latex-to-html <SOURCE.tex> <BIBLIOGRAPHY.bib> <OUT_DIR>");
        process::exit(1);
    }

    let tex_path = PathBuf::from(args().nth(1).unwrap());
    let bib_path = PathBuf::from(args().nth(2).unwrap());
    let out_path = PathBuf::from(args().nth(3).unwrap());

    latex_to_html(tex_path.as_path(), bib_path.as_path(), out_path.as_path());
}

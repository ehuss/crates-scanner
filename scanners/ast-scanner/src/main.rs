//! Scans rust source with an AST visitor.

use anyhow::Result;
use std::path::Path;
use syn::visit::Visit;

fn main() {
    let crates_path = std::env::args()
        .skip(1)
        .next()
        .expect("first argument must be a path to the crates directory");

    crates_scanner::scan_compressed(
        Path::new(&crates_path),
        crates_scanner::Versions::All,
        |path| path.extension().map_or(false, |ext| ext == "rs"),
        scan_rust,
    );
}

fn scan_rust(path: &Path, contents: &str) -> Result<()> {
    let f = match syn::parse_file(&contents) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to parse {path:?}: {e}");
            return Ok(());
        }
    };
    Visitor { file: &contents }.visit_file(&f);
    Ok(())
}

struct Visitor<'a> {
    file: &'a str,
}

impl<'ast> syn::visit::Visit<'ast> for Visitor<'_> {
    fn visit_lit_str(&mut self, i: &'ast syn::LitStr) {
        let s = i.span();
    }
}

//! Scans rust source using tokens.

use anyhow::Result;
use proc_macro2::{TokenStream, TokenTree};
use std::path::Path;
use std::str::FromStr;

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

static CONT_RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();

fn scan_rust(path: &Path, contents: &str) -> Result<()> {
    let tokens = match proc_macro2::TokenStream::from_str(contents) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("failed to parse {path:?}: {e}");
            return Ok(());
        }
    };
    scan(path, tokens)
}

fn scan(path: &Path, tokens: TokenStream) -> Result<()> {
    let cont_re = CONT_RE.get_or_init(|| regex::Regex::new("\\\\\n *\n").unwrap());
    for tt in tokens {
        match tt {
            TokenTree::Group(g) => scan(path, g.stream())?,
            TokenTree::Literal(l) => {
                let s = l.to_string();
                if s.starts_with('"') || s.starts_with("b\"") {
                    if let Some(m) = cont_re.find(&s) {
                        eprintln!("found: {:?}: {:?}", path, m.as_str());
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

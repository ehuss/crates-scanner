//! Scanning `Cargo.toml` with just a toml parser.

use anyhow::{bail, Result};
use std::path::Path;

fn main() {
    let crates_path = std::env::args()
        .skip(1)
        .next()
        .expect("first argument must be a path to the crates directory");

    crates_scanner::scan_compressed(
        Path::new(&crates_path),
        crates_scanner::Versions::All,
        |path| path.file_name().map_or(false, |n| n == "Cargo.toml"),
        check_manifest,
    );
}

fn check_manifest(path: &Path, contents: &str) -> Result<()> {
    let v = match toml::from_str::<toml::Value>(&contents) {
        Ok(v) => v,
        Err(e) => {
            bail!("Failed to parse toml {:?}: {}", path, e);
        }
    };
    let deps = v.get("dependencies");
    check_deps(path, deps);
    if let Some(t) = v.get("target").and_then(|t| t.as_table()) {
        for t_table in t.values() {
            let deps = t_table.as_table().and_then(|t| t.get("dependencies"));
            check_deps(path, deps);
        }
    }
    Ok(())
}

fn check_deps(path: &Path, deps: Option<&toml::Value>) {
    let deps = match deps {
        Some(d) => match d.as_table() {
            Some(t) => t,
            None => {
                println!("{d:?}");
                println!("{path:?} invalid deps syntax?");
                return;
            }
        },
        None => return,
    };
    for dep in deps.values() {
        let t = match dep.as_table() {
            Some(t) => t,
            None => {
                if !dep.is_str() {
                    println!("{path:?} invalid table?");
                }
                return;
            }
        };
        if t.keys().next().is_none() {
            println!("found match: {path:?}");
        }
    }
}

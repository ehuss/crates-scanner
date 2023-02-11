//! Compares manifests to the index.

use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let crates_path = args
        .next()
        .expect("first argument must be a path to the crates directory");
    let index_path = args
        .next()
        .expect("second argument must be a path to the index");

    // let mut all_links = HashMap::new();
    let all_links: HashMap<String, String> = walkdir::WalkDir::new(index_path)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_str().unwrap();
            name != "config.json" && !name.starts_with('.')
        })
        .par_bridge()
        .filter(|entry| entry.as_ref().unwrap().file_type().is_file())
        .flat_map(|entry| {
            let entry = entry.unwrap();
            let contents = std::fs::read_to_string(entry.path()).unwrap();
            contents
                .lines()
                .filter_map(|line| {
                    let v = serde_json::from_str::<serde_json::Value>(line).unwrap();
                    v.get("links").map(|links| {
                        let key = format!(
                            "{}-{}",
                            v["name"].as_str().unwrap(),
                            v["vers"].as_str().unwrap()
                        );
                        (key, links.as_str().unwrap().to_string())
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    eprintln!("found {} crates with links", all_links.len());

    crates_scanner::scan_compressed(
        Path::new(&crates_path),
        crates_scanner::Versions::All,
        |path| path.file_name().map_or(false, |n| n == "Cargo.toml"),
        |path, contents| check_manifest(path, contents, &all_links),
    );
}

fn check_manifest(path: &Path, contents: &str, all_links: &HashMap<String, String>) -> Result<()> {
    let v = match toml::from_str::<toml::Value>(&contents) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to parse toml {:?}: {}", path, e);
            return Ok(());
        }
    };
    let pkg_str = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let index_links = all_links.get(pkg_str);
    let manifest_links = v
        .get("package")
        .unwrap_or_else(|| v.get("project").unwrap())
        .get("links");
    match (manifest_links, index_links) {
        (Some(_), Some(_)) => {}
        (Some(_), None) => println!("{pkg_str} has manifest, missing registry"),
        (None, Some(_)) => println!("{pkg_str} has registry, not manifest!?"),
        (None, None) => {}
    }
    Ok(())
}

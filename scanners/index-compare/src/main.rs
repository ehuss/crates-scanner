//! Compares manifests to the index.

use std::ffi::OsStr;
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

    let index: HashMap<String, _> = walkdir::WalkDir::new(index_path)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_str().unwrap();
            name != "config.json" && !name.starts_with('.')
        })
        .par_bridge()
        .filter(|entry| entry.as_ref().unwrap().file_type().is_file())
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let contents = std::fs::read_to_string(entry.path()).unwrap();
            let versions = contents
                .lines()
                .map(|line| {
                    serde_json::from_str::<serde_json::Value>(line).unwrap()
                })
                .collect::<Vec<_>>();
            if versions.is_empty() {
                return None
            } else {
                Some((versions[0]["name"].as_str().unwrap().to_string(), versions))
            }
        })
        .collect();
    eprintln!("found {} crates", index.len());

    crates_scanner::scan_compressed(
        Path::new(&crates_path),
        crates_scanner::Versions::All,
        |path| path.file_name().map_or(false, |n| n == "Cargo.toml"),
        |crate_filename, path, contents| check_manifest(crate_filename, path, contents, &index),
    );
}

// "sysinfo-0.0.2.crate" "sysinfo-0.0.2/Cargo.toml"

fn check_manifest(crate_filename: &OsStr, path: &Path, contents: &str, index: &HashMap<String, Vec<serde_json::Value>>) -> Result<()> {
    // eprintln!("{crate_filename:?} {path:?}", );
    let crate_filename = crate_filename.to_str().unwrap();
    let no_ext = &crate_filename[..crate_filename.len()-6];
    let inside_path = path.components().next().unwrap();
    match inside_path {
        std::path::Component::Normal(n) => {
            if n != no_ext {
                eprintln!("inside component does not match crate filename {n:?} != {no_ext:?}");
            }
        }
        _ => {panic!("unexpected component");}
    }
    let v = match toml::from_str::<toml::Value>(&contents) {
        Ok(v) => v,
        Err(_e) => {
            eprintln!("failed to parse toml {:?}", path);
            return Ok(());
        }
    };
    let package = v.get("package").unwrap_or_else(|| v.get("project").unwrap());
    let name = package["name"].as_str().unwrap();
    let version = package["version"].as_str().unwrap();
    let index_versions = match index.get(name) {
        Some(v) => v,
        None => {
            eprintln!("couldn't find index entry that matches package.name=\"{name:?}\" in {crate_filename:?}");
            return Ok(());
        }
    };
    let index_entry = match index_versions.iter().find(|vers| vers["vers"].as_str().unwrap() == version) {
        Some(e) => e,
        None => {
            eprintln!("couldn't find index entry for {name:?} with version=\"{version:?}\"");
            let vs: Vec<_> = index_versions.iter().map(|v| v["vers"].as_str().unwrap()).collect();
            eprintln!("versions available: {vs:?}");
            return Ok(());
        }
    };
    if index_entry.get("rust_version") != package.get("rust_version") {
        eprintln!("out of sync: rust_version ({:?} != {:?}) — {name} {version}",
            index_entry.get("rust_version"), package.get("rust_version"));
    }
    // features
    let index_deps = index_entry["deps"].as_array().unwrap();


    Ok(())
}

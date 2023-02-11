//! Counts number of times a crate shows up in build-dependencies.

use anyhow::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

fn main() {
    let crates_path = std::env::args()
        .skip(1)
        .next()
        .expect("first argument must be a path to the crates directory");
    let result = Mutex::new(HashMap::<String, u32>::new());

    crates_scanner::scan_compressed(
        Path::new(&crates_path),
        crates_scanner::Versions::Latest,
        |path| path.file_name().map_or(false, |n| n == "Cargo.toml"),
        |_path, contents| {
            let v = toml::from_str::<toml::Value>(contents)?;
            if let Some(deps) = v.get("build-dependencies") {
                // TODO: This doesn't properly handle renames.
                let t = deps.as_table().unwrap();
                let mut map = result.lock().unwrap();
                for key in t.keys().cloned() {
                    *map.entry(key).or_default() += 1;
                }
            }
            Ok(())
        },
    );
    let mut x: Vec<_> = result.into_inner().unwrap().into_iter().collect();
    x.sort_unstable_by_key(|(_k, v)| *v);
    for (key, value) in x {
        println!("{key}: {value}");
    }
}

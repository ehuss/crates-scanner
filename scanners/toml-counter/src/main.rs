//! Counts number of times a crate shows up in build-dependencies.

use std::collections::HashSet;
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
        crates_scanner::Versions::All,
        |path| path.file_name().map_or(false, |n| n == "Cargo.toml"),
        |crate_path, _entry_path, contents| {
            let v = toml::from_str::<toml::Value>(contents)?;
            let package_edition = v.get("package")
                .and_then(|p| p.get("edition"))
                .map_or("2015", |e| e.as_str().unwrap());

            let mut map = result.lock().unwrap();
            let crate_name = crate_path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            *map.entry(crate_name)
                .or_default()
                += 1;
            // if let Some(deps) = v.get("build-dependencies") {
            //     // TODO: This doesn't properly handle renames.
            //     let t = deps.as_table().unwrap();
            //     let mut map = result.lock().unwrap();
            //     for key in t.keys().cloned() {
            //         *map.entry(key).or_default() += 1;
            //     }
            // }
            Ok(())
        },
    );
    let mut reverse: HashMap<u32, u32> = HashMap::new();
    for (_, v) in result.into_inner().unwrap().into_iter() {
        *reverse.entry(v)
            .or_default()
            += 1;
    }

    for (key, value) in reverse {
        println!("{key:?}: {value}");
    }
}

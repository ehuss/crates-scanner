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
        check_parse,
    );
}

fn check_parse(crate_path: &Path, _entry_path: &Path, contents: &str) -> Result<()> {
    // General parse check
    let new_value = toml::de::DeTable::parse(&contents);
    let old_value = toml_v08::from_str::<toml::Value>(&contents);
    match (old_value.is_ok(), new_value.is_ok()) {
        (false, true) => {
            eprintln!(
                "{crate_path:?} parsing succeeded when it previously failed: {old_value:?}\ncontents: {contents}");
        }
        (true, false) => {
            eprintln!(
                "{crate_path:?} parsing failed when it previously succeeded: {new_value:?}\ncontents: {contents}");
        }
        _ => {}
    }

    // Deserialization check
    let new_value = toml::from_str::<cargo_util_schemas::manifest::TomlManifest>(&contents);
    let old_value = toml_v08::from_str::<cargo_util_schemas::manifest::TomlManifest>(&contents);
    match (old_value.is_ok(), new_value.is_ok()) {
        (false, true) => {
            eprintln!(
                "{crate_path:?} deserialization succeeded when it previously failed: {old_value:?}\ncontents: {contents}");
        }
        (true, false) => {
            eprintln!(
                "{crate_path:?} deserialization failed when it previously succeeded: {new_value:?}\ncontents: {contents}");
        }
        _ => {}

    }

    Ok(())
}

fn check_tab(crate_path: &Path, _entry_path: &Path, contents: &str) -> Result<()> {
    let v = match toml::from_str::<toml::Value>(&contents) {
        Ok(v) => v,
        Err(_e) => {
            bail!("Failed to parse toml {:?}", crate_path);
        }
    };
    check_tab_v(crate_path, contents, &v);
    Ok(())
}

fn check_tab_v(path: &Path, contents: &str, v: &toml::Value) {
    match v {
        toml::Value::String(s) => {
            if s.contains('\t') && contents.contains('\t') {
                eprintln!("{path:?}: {s:?}", );
            }
        }
        toml::Value::Array(a) => {
            for v in a {
                check_tab_v(path, contents, v);
            }
        }
        toml::Value::Table(t) => {
            for v in t.values() {
                check_tab_v(path, contents, v);
            }
        }
        _ => {}
    }

}

fn check_manifest(crate_path: &Path, _entry_path: &Path, contents: &str) -> Result<()> {
    let v = match toml::from_str::<toml::Value>(&contents) {
        Ok(v) => v,
        Err(e) => {
            bail!("Failed to parse toml {:?}: {}", crate_path, e);
        }
    };

    let package_edition = v.get("package")
        .and_then(|p| p.get("edition"))
        .map_or("2015", |e| e.as_str().unwrap());

    let check_target = |name, t: &toml::Value| {
        let Some(t) = t.as_table() else { return };
        if let Some(edition) = t.get("edition") {
            if edition.as_str().unwrap() != package_edition {
                eprintln!("{crate_path:?} {name} sets edition to {edition} (package is {package_edition})");
            }
        }
    };

    let check_targets = |name| {
        if let Some(targets) = v.get(name) {
            let targets = targets.as_array().unwrap();
            for target in targets {
                check_target(name, target);
            }
        }
    };
    if let Some(lib) = v.get("lib") {
        check_target("lib", lib);
    }
    check_targets("bin");
    check_targets("example");
    check_targets("test");
    check_targets("bench");
    // if let Some(features) = v.get("features") {
    //     let features = features.as_table().unwrap();
    //     for (key, value) in features {
    //         if key.contains("derive") {
    //             eprintln!("{key} = {value}");
    //         }
    //     }
    // }
    // let deps = v.get("dependencies");
    // check_deps(path, contents, deps);
    // if let Some(t) = v.get("target").and_then(|t| t.as_table()) {
    //     for t_table in t.values() {
    //         let deps = t_table.as_table().and_then(|t| t.get("dependencies"));
    //         check_deps(path, contents, deps);
    //     }
    // }
    Ok(())
}

fn check_deps(path: &Path, contents: &str, deps: Option<&toml::Value>) {
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
    for (name, dep) in deps {
        let t = match dep.as_table() {
            Some(t) => t,
            None => {
                if !dep.is_str() {
                    println!("{path:?} invalid table?");
                }
                return;
            }
        };
        if let Some(o) = t.get("optional") {
            if name.contains("derive") {
                println!("found {name} in {path:?}");
                println!("{contents}");
            }
        }
        // if t.keys().next().is_none() {
        //     println!("found match: {path:?}");
        // }
    }
}

//! Compares Cargo.lock generated with different versions.

use anyhow::{bail, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let mut args = std::env::args().skip(1);
    let src_path = args
        .next()
        .expect("first argument must be a path to the extracted source directory");
    let cargo_path = args
        .next()
        .expect("second argument must be a path to your locally built cargo");

    crates_scanner::scan_uncompressed(Path::new(&src_path), |path| {
        let lock_path = path.join("Cargo.lock");
        let mut old_lock_path = None;
        if lock_path.exists() {
            old_lock_path = Some(lock_path.with_file_name("Cargo.lock.scan-backup"));
            fs::rename(&lock_path, old_lock_path.as_ref().unwrap()).unwrap();
        }
        let result = gen_and_compare(path, &lock_path, &cargo_path);
        if let Some(original) = &old_lock_path {
            if let Err(e) = fs::rename(original, &lock_path) {
                eprintln!("Failed to move to {original:?} from {lock_path:?}: {e:?}",);
            }
        }
        result
    });
}

fn gen_and_compare(path: &Path, lock_path: &Path, cargo_path: &str) -> Result<()> {
    let output = Command::new("cargo")
        .args(&["generate-lockfile", "-Zno-index-update"])
        .current_dir(path)
        .output()
        .unwrap();
    if output.status.success() {
        let stable_path = lock_path.with_file_name("Cargo.lock.stable");
        fs::rename(&lock_path, &stable_path).unwrap();
        let output = Command::new(&cargo_path)
            .args(&["generate-lockfile", "-Zno-index-update"])
            .current_dir(path)
            .output()
            .unwrap();
        if output.status.success() {
            let stable = fs::read_to_string(&stable_path).unwrap();
            let new = fs::read_to_string(&lock_path).unwrap();
            if stable != new {
                bail!("{:?} is different !!!!", path);
            }
        } else {
            bail!(
                "{:?} new failed:\n{}\n{}",
                path,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        drop(fs::remove_file(stable_path));
    } else {
        eprintln!(
            "{:?} failed:\n{}\n{}",
            path,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

//! This is a tool for extracting the latest version of a `.crate`.

use core::sync::atomic::{AtomicU32, Ordering};
use rayon::prelude::*;
use std::path::Path;
use std::process::Command;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(crates_path) = args.next() else {
        eprintln!("first argument must be a path to the crates directory");
        std::process::exit(1);
    };
    let crates_path = Path::new(&crates_path);
    let Some(output_path) = args.next() else {
        eprintln!("second argument must be a path to write the source files");
        std::process::exit(1);
    };
    let output_path = Path::new(&output_path);

    let latest = crates_scanner::collect_latest_crates(&crates_path);
    eprintln!("total: {}", latest.len());

    let extracted = AtomicU32::new(0);
    let errors = AtomicU32::new(0);

    latest.par_iter().for_each(|path| {
        let rel = path.strip_prefix(crates_path).unwrap();
        let crate_out_path = output_path.join(rel.with_extension(""));
        if crate_out_path.exists() {
            return;
        }
        let parent = crate_out_path.parent().unwrap();
        if parent.exists() {
            // Remove any old versions.
            std::fs::remove_dir_all(&parent).unwrap();
        }
        std::fs::create_dir_all(&parent).unwrap();
        eprintln!("extracting to {crate_out_path:?}");
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(path)
            .arg("-C")
            .arg(&parent)
            .status()
            .unwrap();
        if status.success() {
            extracted.fetch_add(1, Ordering::SeqCst);
        } else {
            errors.fetch_add(1, Ordering::SeqCst);
            eprintln!("error: failed to extract {path:?} into {crate_out_path:?}");
        }
    });
    println!(
        "extracted: {}\n\
        errors: {}\n\
        total: {}",
        extracted.load(Ordering::SeqCst),
        errors.load(Ordering::SeqCst),
        latest.len()
    );
}

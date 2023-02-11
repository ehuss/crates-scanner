//! Runs `cargo tree` on every crate.

use anyhow::bail;
use std::io::Write;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let src_path = args
        .next()
        .expect("first argument must be a path to the extracted source directory");
    let cargo_path = args
        .next()
        .expect("second argument must be a path to your locally built cargo");
    crates_scanner::overdrive(2);

    crates_scanner::scan_uncompressed(Path::new(&src_path), |path| {
        let output = std::process::Command::new(&cargo_path)
            .args(&["tree", "-Zno-index-update"])
            .current_dir(path)
            .output()
            .unwrap();
        if !output.status.success() {
            let stderr = std::str::from_utf8(&output.stderr).unwrap();
            let stdout = std::str::from_utf8(&output.stdout).unwrap();
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("tree_results.txt")
                .unwrap();
            writeln!(
                f,
                "Failed: {path:?}\n---stderr\n{stderr}\n--stdout\n{stdout}\n"
            )
            .unwrap();
            bail!("{path:?} FAILED!!!");
        }

        Ok(())
    });
}

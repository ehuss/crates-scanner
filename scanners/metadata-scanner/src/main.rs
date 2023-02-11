//! Uses cargo_metadata on all crates.

use anyhow::bail;
use cargo_metadata::MetadataCommand;
use std::path::Path;

fn main() {
    let src_path = std::env::args()
        .skip(1)
        .next()
        .expect("first argument must be a path to the extracted source directory");

    crates_scanner::scan_uncompressed(Path::new(&src_path), |path| {
        let meta = match MetadataCommand::new()
            .manifest_path(path.join("Cargo.toml"))
            .no_deps()
            .exec()
        {
            Ok(meta) => meta,
            Err(e) => {
                bail!("could not run metadata for {path:?}: {e}");
            }
        };
        assert_eq!(meta.packages.len(), 1);
        for dep in &meta.packages[0].dependencies {
            if dep.features.iter().any(|f| f.starts_with('_')) {
                println!(
                    "{:?} depends on {} with features: {:?}",
                    path, dep.name, dep.features
                );
            }
        }

        Ok(())
    });
}

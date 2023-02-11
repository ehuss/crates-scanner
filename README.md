# Crates scanners

This contains some scanners for scanning all crates on crates.io.
The `crates-scanner` crate contains some helper functions for scanning all crates.
The `scanners` directory contains various examples that parse TOML files, run `cargo` commands, parse Rust files, etc.

You'll need a clone of https://github.com/rust-lang/crates.io-index/ and use <https://github.com/dtolnay/get-all-crates/> to download all crates (as of 2023-02-10 is about 110GB).

`extract-latest` will uncompress the latest version of every crate into a directory, which can be useful for tools that can't directly work with the compressed files (as of 2023-02-10 is about 58GB).
Be careful not to run any tools that would execute code from the crate.

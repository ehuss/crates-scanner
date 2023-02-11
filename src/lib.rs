use anyhow::Result;
use flate2::read::GzDecoder;
use rayon::prelude::*;
use semver::Version;
use std::collections::hash_map::{Entry, HashMap};
use std::fs::{read_dir, File};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use tar::Archive;

const ERROR: &str = "\x1b[1m\x1b[38;5;9merror\x1b[0m";

#[derive(Copy, Clone)]
pub enum Versions {
    All,
    Latest,
}

/// Scans compressed `.crate` files.
pub fn scan_compressed<Filt, Scan>(
    crates_path: &Path,
    versions: Versions,
    filter: Filt,
    scanner: Scan,
) where
    Filt: Fn(&Path) -> bool + Sync,
    Scan: Fn(&Path, &str) -> Result<()> + Sync,
{
    let paths = match versions {
        Versions::All => collect_all_crates(crates_path),
        Versions::Latest => collect_latest_crates(crates_path),
    };
    eprintln!("scanning {} crates", paths.len());

    let scanned = AtomicU32::new(0);
    let load_errors = AtomicU32::new(0);
    let scan_errors = AtomicU32::new(0);

    paths.par_iter().for_each(|crate_path| {
        let f = GzDecoder::new(File::open(crate_path).unwrap());
        let mut archive = Archive::new(f);
        for entry in archive.entries().unwrap() {
            let mut entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    load_errors.fetch_add(1, Ordering::SeqCst);
                    eprintln!("entry error {crate_path:?}: {e}");
                    break;
                }
            };
            let entry_path = match entry.path() {
                Ok(p) => p,
                Err(e) => {
                    load_errors.fetch_add(1, Ordering::SeqCst);
                    eprintln!("path decode error {crate_path:?}: {e}");
                    break;
                }
            };
            if filter(&entry_path) {
                let mut display_path = PathBuf::from(entry_path.file_name().unwrap());
                display_path.push(entry_path);
                let mut contents = String::new();
                if let Err(e) = entry.read_to_string(&mut contents) {
                    load_errors.fetch_add(1, Ordering::SeqCst);
                    eprintln!("decode error {display_path:?}: {e}");
                    break;
                }
                let progress = scanned.fetch_add(1, Ordering::SeqCst);
                if progress % 10000 == 0 {
                    eprintln!("processed {progress}");
                }

                if let Err(e) = scanner(&display_path, &contents) {
                    eprintln!(
                        "{ERROR} scanning {display_path:?}: {e:?}\n\
                            contents:\n{contents}"
                    );
                    scan_errors.fetch_add(1, Ordering::SeqCst);
                    break;
                }
            }
        }
    });
    println!(
        "load errors: {}\n\
        scan errors: {}\n\
        total: {}",
        load_errors.load(Ordering::SeqCst),
        scan_errors.load(Ordering::SeqCst),
        paths.len()
    );
}

/// Scans uncompressed crates.
///
/// Crates must be first extracted with the extract-latest tool.
pub fn scan_uncompressed<Scan>(src_path: &Path, scanner: Scan)
where
    Scan: Fn(&Path) -> Result<()> + Sync,
{
    let paths = collect_uncompressed_paths(src_path);
    let total = AtomicU32::new(0);
    let scan_errors = AtomicU32::new(0);
    eprintln!("scanning {} crates", paths.len());
    paths.par_iter().for_each(|path| {
        let progress = total.fetch_add(1, Ordering::SeqCst);
        if progress % 10000 == 0 {
            eprintln!("processed {progress}/{}", paths.len());
        }
        if let Err(e) = scanner(&path) {
            eprintln!("{ERROR} scanning {path:?}: {e:?}");
            scan_errors.fetch_add(1, Ordering::SeqCst);
        }
    });
    println!(
        "scan errors: {}\n\
        total: {}",
        scan_errors.load(Ordering::SeqCst),
        paths.len()
    );
}

fn collect_uncompressed_paths(src_path: &Path) -> Vec<PathBuf> {
    read_dir(src_path)
        .unwrap()
        .par_bridge()
        .flat_map(|entry| {
            let entry = entry.unwrap();
            let file_name = entry.file_name();
            read_dir(src_path.join(&file_name))
                .unwrap()
                .par_bridge()
                .flat_map(move |entry| {
                    read_dir(entry.unwrap().path())
                        .unwrap()
                        .flat_map(|entry| {
                            if file_name == "1" || file_name == "2" {
                                vec![entry.unwrap().path()]
                            } else {
                                read_dir(entry.unwrap().path())
                                    .unwrap()
                                    .map(|entry| entry.unwrap().path())
                                    .collect()
                            }
                        })
                        .collect::<Vec<_>>()
                })
        })
        .collect()
}

fn collect_all_crates(crates_path: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(crates_path)
        .into_iter()
        .filter_map(|entry| {
            let e = entry.unwrap();
            if e.file_name().to_str().unwrap().ends_with(".crate") {
                Some(e.path().to_path_buf())
            } else {
                None
            }
        })
        .collect()
}

pub fn collect_latest_crates(crates_path: &Path) -> Vec<PathBuf> {
    let mut versions = HashMap::new();

    walkdir::WalkDir::new(crates_path)
        .into_iter()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .ends_with(".crate")
        })
        .for_each(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let crate_name = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let version = &file_name[crate_name.len() + 1..file_name.len() - 6];
            let version = Version::parse(version).unwrap();
            let rel_path = path.strip_prefix(&crates_path).unwrap();
            match versions.entry(crate_name.clone()) {
                Entry::Vacant(e) => {
                    e.insert((rel_path.to_owned(), version));
                }
                Entry::Occupied(mut e) => {
                    if version > e.get().1 {
                        e.insert((rel_path.to_owned(), version));
                    }
                }
            }
        });

    versions
        .into_iter()
        .map(|(_name, (path, _count))| crates_path.join(path))
        .collect()
}

pub fn overdrive(n: usize) {
    let n = std::thread::available_parallelism().unwrap().get() * n;
    rayon::ThreadPoolBuilder::new()
        .num_threads(n)
        .build_global()
        .unwrap();
}

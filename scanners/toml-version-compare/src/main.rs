use anyhow::{bail, format_err, Result};
use std::path::Path;

fn main() {
    let crates_path = std::env::args()
        .skip(1)
        .next()
        .expect("first argument must be a path to the crates directory");

    crates_scanner::scan_compressed(
        Path::new(&crates_path),
        crates_scanner::Versions::All,
        |path| {
            path.extension().map_or(false, |e| e == "toml")
                || path.file_name().map_or(false, |n| n == "Cargo.lock")
        },
        check_parse,
    );
}

fn check_parse(path: &Path, contents: &str) -> Result<()> {
    let v7 = match toml7::from_str::<toml7::Value>(contents) {
        Ok(v) => v,
        Err(e) => {
            // eprintln!("failed to parse {path:?}: {e}");
            if toml5::from_str::<toml5::Value>(contents).is_ok() {
                bail!("PARSE DIFFERENCE toml 0.5 pass, 0.7 fail {path:?}: {e}");
            }
            return Ok(());
        }
    };
    let v5 = toml5::from_str::<toml5::Value>(contents)
        .map_err(|e| format_err!("v5 failed {path:?}: {e}"))?;
    if !compare(&v5, &v7) {
        bail!("compare mismatch {path:?}");
    }
    Ok(())
}

fn compare(v5: &toml5::Value, v7: &toml7::Value) -> bool {
    match (v5, v7) {
        (toml5::Value::String(s5), toml7::Value::String(s7)) => return s5 == s7,
        (toml5::Value::Integer(s5), toml7::Value::Integer(s7)) => return s5 == s7,
        (toml5::Value::Float(s5), toml7::Value::Float(s7)) => return s5 == s7,
        (toml5::Value::Boolean(s5), toml7::Value::Boolean(s7)) => return s5 == s7,
        (toml5::Value::Datetime(s5), toml7::Value::Datetime(s7)) => {
            return s5.to_string() == s7.to_string()
        }
        (toml5::Value::Array(s5), toml7::Value::Array(s7)) => {
            if s5.len() != s7.len() {
                return false;
            }
            for (s5, s7) in std::iter::zip(s5, s7) {
                if !compare(s5, s7) {
                    return false;
                }
            }
            return true;
        }
        (toml5::Value::Table(s5), toml7::Value::Table(s7)) => {
            if s5.len() != s7.len() {
                return false;
            }
            for (key, value) in s5 {
                let s7_val = s7.get(key).unwrap();
                compare(value, s7_val);
            }
            return true;
        }
        _ => return false,
    }
}

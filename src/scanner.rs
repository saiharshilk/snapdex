use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "heic"];

pub fn scan_folder(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(folder).min_depth(1).max_depth(1) {
        let entry = entry.map_err(io::Error::other)?;
        let path = entry.into_path();
        if !path.is_file() || is_history_file(&path) || is_snapdex_name(&path) {
            continue;
        }

        let is_image = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|extension| {
                IMAGE_EXTENSIONS
                    .iter()
                    .any(|allowed| extension.eq_ignore_ascii_case(allowed))
            })
            .unwrap_or(false);
        if is_image {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

pub fn is_snapdex_name(path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(OsStr::to_str) else {
        return false;
    };
    let bytes = stem.as_bytes();
    bytes.len() >= 11
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'_'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn is_history_file(path: &Path) -> bool {
    path.file_name().and_then(OsStr::to_str) == Some(".snapdex_history.json")
}

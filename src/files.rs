use std::{path::Path, ffi::OsStr};

/// Matches file extension of `path` 
pub(crate) fn has_extension_single(path: &Path, ext: &str) -> bool {
    // ensure file extension does not start with '.'
    let ext = ext.trim_start_matches(".");
    if let Some(path_ext) = path.extension() {
        return path_ext.to_ascii_lowercase() == OsStr::new(&ext).to_ascii_lowercase()
    }
    false
}

/// Matches `path` with extensions in `exts` and returns
/// the first match as a `String`.
pub(crate) fn has_extension(path: &Path, exts: &[&str]) -> Option<String> {
    for ext in exts {
        if has_extension_single(path, ext) {
            return Some(ext.to_string())
        }
    }
    None
}

/// Returns file extension as lower case string.
pub(crate) fn fileext_to_lcstring(path: &Path) -> Option<String> {
    Some(path.extension()?.to_str()?.to_ascii_lowercase())
}

/// Returns filestem as a `String`.
pub(crate) fn filestem_to_string(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_str()?.to_string())
}

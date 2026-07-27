use std::path::{Component, Path, PathBuf};

/// Canonicalize a path, stripping the `\\?\` verbatim prefix on Windows.
pub fn safe_canonicalize(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).map_or_else(
        |_| {
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(path)
            };

            for ancestor in absolute.ancestors().skip(1) {
                if let Ok(canonical_ancestor) = std::fs::canonicalize(ancestor)
                    && let Ok(suffix) = absolute.strip_prefix(ancestor)
                {
                    let combined = canonical_ancestor.join(suffix);
                    return strip_unc_prefix(normalize_dot_segments(&combined));
                }
            }

            strip_unc_prefix(normalize_dot_segments(&absolute))
        },
        strip_unc_prefix,
    )
}

fn normalize_dot_segments(path: &Path) -> PathBuf {
    use std::ffi::{OsStr, OsString};

    let mut out = PathBuf::new();
    let mut normals: Vec<OsString> = Vec::new();
    let mut has_prefix = false;
    let mut has_root = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                out.push(prefix.as_os_str());
                has_prefix = true;
            }
            Component::RootDir => {
                out.push(component.as_os_str());
                has_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => match normals.last() {
                Some(last) if last.as_os_str() != OsStr::new("..") => {
                    normals.pop();
                }
                _ => {
                    if !has_root && !has_prefix {
                        normals.push(OsString::from(".."));
                    }
                }
            },
            Component::Normal(part) => normals.push(part.to_os_string()),
        }
    }

    for part in normals {
        out.push(part);
    }

    out
}

/// Strip the `\\?\` or `//?/` verbatim prefix from a path on Windows.
pub fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            if let Some(unc) = stripped.strip_prefix("UNC") && unc.starts_with('\\') {
                return PathBuf::from(format!(r"\{unc}"));
            }
            return PathBuf::from(stripped);
        }
        if let Some(stripped) = s.strip_prefix("//?/") {
            return PathBuf::from(stripped);
        }
    }
    path
}

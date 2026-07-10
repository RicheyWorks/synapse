use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::error::SynapseError;

/// Extensions accepted by `import_asset`. Kept intentionally narrow (real
/// image formats only) rather than accepting anything, since this becomes
/// part of the vault and gets carried through backups/exports.
const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];

/// Copies `source_path` into `assets_dir` under a fresh UUID-based filename
/// (original extension preserved), so an image card no longer depends on the
/// source file's original location — it survives the source being moved,
/// renamed, or deleted after import. Returns a path relative to the app data
/// directory (e.g. `"assets/<uuid>.png"`) for storage in `CardContent::Image::path`.
pub fn import_asset(source_path: &Path, assets_dir: &Path) -> Result<String, SynapseError> {
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !ALLOWED_IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return Err(SynapseError::InvalidOperation(format!(
            "unsupported image type \".{ext}\" (allowed: {})",
            ALLOWED_IMAGE_EXTENSIONS.join(", ")
        )));
    }

    let bytes = fs::read(source_path).map_err(|source| SynapseError::Io {
        path: source_path.to_path_buf(),
        source,
    })?;

    fs::create_dir_all(assets_dir).map_err(|source| SynapseError::Io {
        path: assets_dir.to_path_buf(),
        source,
    })?;

    let filename = format!("{}.{ext}", Uuid::new_v4());
    let dest = assets_dir.join(&filename);
    fs::write(&dest, &bytes).map_err(|source| SynapseError::Io {
        path: dest.clone(),
        source,
    })?;

    Ok(format!("assets/{filename}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_a_valid_image_and_returns_a_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("original.png");
        fs::write(&source, b"not a real png, just test bytes").unwrap();

        let assets_dir = dir.path().join("assets");
        let relative = import_asset(&source, &assets_dir).unwrap();

        assert!(relative.starts_with("assets/"));
        assert!(relative.ends_with(".png"));

        let filename = relative.strip_prefix("assets/").unwrap();
        let copied_path = assets_dir.join(filename);
        assert!(copied_path.exists());
        assert_eq!(fs::read(&copied_path).unwrap(), b"not a real png, just test bytes");
    }

    #[test]
    fn two_imports_of_the_same_source_get_distinct_filenames() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("original.jpg");
        fs::write(&source, b"jpeg bytes").unwrap();

        let assets_dir = dir.path().join("assets");
        let first = import_asset(&source, &assets_dir).unwrap();
        let second = import_asset(&source, &assets_dir).unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn rejects_non_image_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("script.exe");
        fs::write(&source, b"binary").unwrap();

        let result = import_asset(&source, &dir.path().join("assets"));
        assert!(matches!(result, Err(SynapseError::InvalidOperation(_))));
    }

    #[test]
    fn missing_source_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("does-not-exist.png");

        let result = import_asset(&source, &dir.path().join("assets"));
        assert!(matches!(result, Err(SynapseError::Io { .. })));
    }
}

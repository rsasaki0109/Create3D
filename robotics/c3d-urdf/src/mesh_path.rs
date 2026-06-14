use std::path::{Path, PathBuf};

use crate::error::{UrdfError, UrdfResult};

/// Resolve a URDF mesh filename against the URDF package directory.
///
/// Supports `package://`, `file://`, absolute paths, and paths relative to the URDF file.
pub fn resolve_urdf_mesh_path(filename: &str, package_path: Option<&Path>) -> UrdfResult<PathBuf> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err(UrdfError::Invalid("mesh filename is empty".into()));
    }

    if let Some(path) = trimmed.strip_prefix("file://") {
        return Ok(decode_file_uri(path));
    }

    if let Some(rest) = trimmed.strip_prefix("package://") {
        let path_part = rest
            .split_once('/')
            .map(|(_, path)| path)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| UrdfError::Invalid(format!("invalid package mesh URI `{trimmed}`")))?;
        return resolve_relative_mesh_path(path_part, package_path);
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        return Err(UrdfError::Invalid(format!(
            "mesh file not found: `{}`",
            path.display()
        )));
    }

    resolve_relative_mesh_path(trimmed, package_path)
}

fn decode_file_uri(path: &str) -> PathBuf {
    if cfg!(windows) {
        return PathBuf::from(path.trim_start_matches('/'));
    }
    PathBuf::from(path)
}

fn resolve_relative_mesh_path(relative: &str, package_path: Option<&Path>) -> UrdfResult<PathBuf> {
    let Some(base) = package_path else {
        return Err(UrdfError::Invalid(format!(
            "cannot resolve mesh `{relative}` without a URDF package path"
        )));
    };

    let mut candidates = vec![base.join(relative)];
    if let Some(parent) = base.parent() {
        candidates.push(parent.join(relative));
    }

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(UrdfError::Invalid(format!(
        "mesh file not found for `{relative}` (searched near `{}`)",
        base.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_relative_mesh_next_to_urdf() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mesh_path = temp.path().join("link.glb");
        fs::write(&mesh_path, b"stub").expect("write mesh");

        let resolved = resolve_urdf_mesh_path("link.glb", Some(temp.path())).expect("resolve");
        assert_eq!(resolved, mesh_path);
    }

    #[test]
    fn resolves_package_uri_from_parent_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let package_root = temp.path().join("my_robot");
        let urdf_dir = package_root.join("urdf");
        let mesh_dir = package_root.join("meshes");
        fs::create_dir_all(&urdf_dir).expect("urdf dir");
        fs::create_dir_all(&mesh_dir).expect("mesh dir");
        let mesh_path = mesh_dir.join("base.stl");
        fs::write(&mesh_path, b"stub").expect("write mesh");

        let resolved =
            resolve_urdf_mesh_path("package://my_robot/meshes/base.stl", Some(&urdf_dir))
                .expect("resolve package uri");
        assert_eq!(resolved, mesh_path);
    }

    #[test]
    fn rejects_missing_mesh_with_context() {
        let temp = tempfile::tempdir().expect("tempdir");
        let err = resolve_urdf_mesh_path("meshes/missing.stl", Some(temp.path()))
            .expect_err("missing mesh");
        assert!(matches!(err, UrdfError::Invalid(_)));
    }
}

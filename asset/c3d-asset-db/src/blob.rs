use std::fs;
use std::path::{Path, PathBuf};

use crate::{AssetError, AssetResult, ContentHash};

/// Filesystem-backed immutable blob storage.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    /// Open or create blob storage under `assets/blobs`.
    pub fn new(project_root: impl AsRef<Path>) -> AssetResult<Self> {
        let root = project_root.as_ref().join("assets/blobs");
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Store bytes when absent and return the content hash.
    pub fn put_bytes(&self, bytes: &[u8]) -> AssetResult<ContentHash> {
        let hash = ContentHash::of_bytes(bytes);
        let path = self.path_for(hash);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)?;
        }
        Ok(hash)
    }

    /// Read blob bytes by content hash.
    pub fn get_bytes(&self, hash: ContentHash) -> AssetResult<Vec<u8>> {
        let path = self.path_for(hash);
        fs::read(path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AssetError::BlobNotFound(hash.to_hex())
            } else {
                AssetError::Io(err)
            }
        })
    }

    /// Returns true when the blob exists on disk.
    pub fn contains(&self, hash: ContentHash) -> bool {
        self.path_for(hash).is_file()
    }

    fn path_for(&self, hash: ContentHash) -> PathBuf {
        let hex = hash.to_hex();
        let prefix = &hex[..2];
        self.root.join(prefix).join(hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_is_content_addressed() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = BlobStore::new(temp.path()).expect("open store");
        let hash = store.put_bytes(b"mesh-data").expect("put blob");
        assert!(store.contains(hash));
        assert_eq!(
            store.get_bytes(hash).expect("read blob"),
            b"mesh-data".to_vec()
        );
    }
}

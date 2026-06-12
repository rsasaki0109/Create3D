use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use c3d_core::AssetId;

use crate::{AssetError, AssetIndexDocument, AssetKind, AssetRecord, AssetResult, BlobStore};

/// Project asset database backed by an index file and content-addressed blobs.
#[derive(Debug, Clone)]
pub struct AssetDb {
    root: PathBuf,
    blobs: BlobStore,
    records: HashMap<AssetId, AssetRecord>,
}

impl AssetDb {
    /// Load or initialize an asset database under a project root directory.
    pub fn open(project_root: impl AsRef<Path>) -> AssetResult<Self> {
        let root = project_root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("assets"))?;
        let blobs = BlobStore::new(&root)?;
        let index_path = Self::index_path(&root);
        let records = if index_path.is_file() {
            let json = fs::read_to_string(&index_path)?;
            let document: AssetIndexDocument = serde_json::from_str(&json)
                .map_err(|err| AssetError::Serialization(err.to_string()))?;
            document
                .assets
                .into_iter()
                .map(|record| (record.id, record))
                .collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            root,
            blobs,
            records,
        })
    }

    /// Returns the project root directory.
    pub fn project_root(&self) -> &Path {
        &self.root
    }

    /// Borrow blob storage.
    pub fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    /// Iterate asset records in stable id order.
    pub fn records(&self) -> impl Iterator<Item = &AssetRecord> {
        let mut records: Vec<_> = self.records.values().collect();
        records.sort_by_key(|record| record.id);
        records.into_iter()
    }

    /// Lookup an asset record by id.
    pub fn get(&self, asset_id: AssetId) -> Option<&AssetRecord> {
        self.records.get(&asset_id)
    }

    /// Read blob bytes for an asset id.
    pub fn read_blob(&self, asset_id: AssetId) -> AssetResult<Vec<u8>> {
        let record = self
            .records
            .get(&asset_id)
            .ok_or(AssetError::NotFound(asset_id))?;
        self.blobs.get_bytes(record.content_hash)
    }

    /// Insert a new asset record and store its blob bytes.
    pub fn insert(
        &mut self,
        id: AssetId,
        kind: AssetKind,
        name: impl Into<String>,
        bytes: &[u8],
        mime_type: Option<String>,
    ) -> AssetResult<AssetRecord> {
        let content_hash = self.blobs.put_bytes(bytes)?;
        let record = AssetRecord {
            id,
            kind,
            content_hash,
            name: name.into(),
            mime_type,
        };
        self.records.insert(id, record.clone());
        Ok(record)
    }

    /// Replace blob bytes for an existing asset record.
    pub fn replace_blob(&mut self, id: AssetId, bytes: &[u8]) -> AssetResult<AssetRecord> {
        let record = self.records.get_mut(&id).ok_or(AssetError::NotFound(id))?;
        record.content_hash = self.blobs.put_bytes(bytes)?;
        Ok(record.clone())
    }

    /// Persist the asset index to disk.
    pub fn save(&self) -> AssetResult<()> {
        let mut assets: Vec<_> = self.records.values().cloned().collect();
        assets.sort_by_key(|record| record.id);
        let document = AssetIndexDocument { version: 1, assets };
        let json = serde_json::to_string_pretty(&document)
            .map_err(|err| AssetError::Serialization(err.to_string()))?;
        fs::write(Self::index_path(&self.root), json)?;
        Ok(())
    }

    fn index_path(root: &Path) -> PathBuf {
        root.join("assets/index.c3dassetdb")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_index() {
        let temp = tempfile::tempdir().expect("temp dir");
        let asset_id = AssetId::new();
        {
            let mut db = AssetDb::open(temp.path()).expect("open db");
            db.insert(
                asset_id,
                AssetKind::Mesh,
                "cube",
                br#"{"positions":[]}"#,
                Some("application/json".into()),
            )
            .expect("insert asset");
            db.save().expect("save db");
        }

        let db = AssetDb::open(temp.path()).expect("reload db");
        let record = db.get(asset_id).expect("asset record");
        assert_eq!(record.name, "cube");
        assert_eq!(
            db.read_blob(asset_id).expect("read blob"),
            br#"{"positions":[]}"#.to_vec()
        );
    }
}

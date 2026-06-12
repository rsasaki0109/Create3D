use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{ProjectError, ProjectResult};
use crate::Project;

/// Metadata for a crash-safe autosave snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySnapshot {
    /// Unix timestamp in milliseconds when the autosave was written.
    pub saved_at_ms: u64,
    /// Relative path to the autosaved scene document.
    pub scene_path: String,
}

impl RecoverySnapshot {
    /// Load recovery metadata from a project root if present.
    pub fn load(root: impl AsRef<Path>) -> ProjectResult<Option<Self>> {
        let path = root.as_ref().join(".recovery/recovery.json");
        if !path.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(path)?;
        serde_json::from_str(&text)
            .map_err(|err: serde_json::Error| ProjectError::Recovery(err.to_string()))
            .map(Some)
    }
}

impl Project {
    /// Write a crash-safe autosave snapshot of the current scene.
    pub fn write_autosave(&self) -> ProjectResult<RecoverySnapshot> {
        let recovery_dir = self.root().join(".recovery");
        fs::create_dir_all(&recovery_dir)?;

        let scene_rel = "autosave.c3dscene.json";
        let scene_path = recovery_dir.join(scene_rel);
        let json = self.scene().to_json()?;
        let temp = recovery_dir.join("autosave.c3dscene.json.tmp");
        fs::write(&temp, json)?;
        fs::rename(temp, &scene_path)?;

        let snapshot = RecoverySnapshot {
            saved_at_ms: now_ms(),
            scene_path: scene_rel.into(),
        };
        let meta_temp = recovery_dir.join("recovery.json.tmp");
        let meta = serde_json::to_string_pretty(&snapshot)
            .map_err(|err: serde_json::Error| ProjectError::Recovery(err.to_string()))?;
        fs::write(&meta_temp, meta)?;
        fs::rename(meta_temp, recovery_dir.join("recovery.json"))?;
        Ok(snapshot)
    }

    /// Returns recovery metadata when an autosave exists.
    pub fn recovery_snapshot(&self) -> ProjectResult<Option<RecoverySnapshot>> {
        RecoverySnapshot::load(self.root())
    }

    /// Restore the main scene from the latest autosave snapshot.
    pub fn recover_from_autosave(&mut self) -> ProjectResult<RecoverySnapshot> {
        let snapshot = self
            .recovery_snapshot()?
            .ok_or_else(|| ProjectError::Recovery("no autosave snapshot found".into()))?;
        let autosave_path = self.root().join(".recovery").join(&snapshot.scene_path);
        if !autosave_path.is_file() {
            return Err(ProjectError::Recovery(format!(
                "autosave scene missing at {}",
                autosave_path.display()
            )));
        }
        let json = fs::read_to_string(autosave_path)?;
        *self.scene_mut() = c3d_scene_doc::SceneDoc::from_json(&json)?;
        self.save()?;
        Ok(snapshot)
    }

    /// Remove recovery artifacts after a successful manual save or dismiss.
    pub fn clear_recovery(&self) -> ProjectResult<()> {
        let recovery_dir = self.root().join(".recovery");
        if recovery_dir.is_dir() {
            fs::remove_dir_all(recovery_dir)?;
        }
        Ok(())
    }

    /// Returns true when an autosave is newer than the main scene file.
    pub fn recovery_is_newer(&self) -> ProjectResult<bool> {
        let Some(snapshot) = self.recovery_snapshot()? else {
            return Ok(false);
        };
        let autosave = self.root().join(".recovery").join(&snapshot.scene_path);
        if !autosave.is_file() {
            return Ok(false);
        }
        let autosave_mtime = fs::metadata(&autosave)?.modified()?;
        let scene_path = self.root().join(self.manifest().main_scene.clone());
        let scene_mtime = fs::metadata(&scene_path)?.modified()?;
        Ok(autosave_mtime > scene_mtime)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::UlidGenerator;
    use c3d_scene_ops::{apply_operations, SceneOperation};
    use c3d_scene_schema::{Name, Transform};

    #[test]
    fn autosave_round_trips_and_recovery_restores_scene() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut project = Project::create(temp.path(), "recovery-test").expect("create");
        let mut ids = UlidGenerator::new();
        let entity_id = ids.next_entity_id();
        apply_operations(
            project.scene_mut(),
            &[SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(Name::new("Recovered")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
                robot_root: None,
                robot_link: None,
                robot_joint: None,
            }],
        )
        .expect("apply");
        project.save().expect("save");
        project.write_autosave().expect("autosave");

        *project.scene_mut() = c3d_scene_doc::SceneDoc::new();
        assert_eq!(project.scene().entity_count(), 0);

        project.recover_from_autosave().expect("recover");
        assert_eq!(project.scene().entity_count(), 1);
        project.clear_recovery().expect("clear");
        assert!(project.recovery_snapshot().expect("snapshot").is_none());
    }
}

use c3d_scene_doc::{SceneDoc, SceneResult};

use crate::apply::apply_operations;
use crate::{SceneOperation, Transaction};

/// Record of an applied transaction kept on the undo/redo stacks.
#[derive(Debug, Clone, PartialEq)]
struct AppliedTransaction {
    id: c3d_core::TransactionId,
    forward: Vec<SceneOperation>,
    inverse: Vec<SceneOperation>,
}

/// Validates, applies, undoes, and redoes scene transactions.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionManager {
    scene: SceneDoc,
    undo_stack: Vec<AppliedTransaction>,
    redo_stack: Vec<AppliedTransaction>,
}

impl TransactionManager {
    /// Create a manager from an existing scene document.
    pub fn new(scene: SceneDoc) -> Self {
        Self {
            scene,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Borrow the authoritative scene document.
    pub fn scene(&self) -> &SceneDoc {
        &self.scene
    }

    /// Mutably borrow the scene document.
    ///
    /// Direct mutation bypasses undo/redo. Prefer [`Self::apply`](Self::apply).
    pub fn scene_mut(&mut self) -> &mut SceneDoc {
        &mut self.scene
    }

    /// Returns true when undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true when redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Apply a transaction and push it onto the undo stack.
    pub fn apply(&mut self, transaction: Transaction) -> SceneResult<()> {
        let inverse = apply_operations(&mut self.scene, &transaction.operations)?;
        self.undo_stack.push(AppliedTransaction {
            id: transaction.id,
            forward: transaction.operations,
            inverse,
        });
        self.redo_stack.clear();
        Ok(())
    }

    /// Undo the most recent transaction.
    pub fn undo(&mut self) -> SceneResult<()> {
        let Some(record) = self.undo_stack.pop() else {
            return Ok(());
        };

        apply_operations(&mut self.scene, &record.inverse)?;
        self.redo_stack.push(record);
        Ok(())
    }

    /// Redo the most recently undone transaction.
    pub fn redo(&mut self) -> SceneResult<()> {
        let Some(record) = self.redo_stack.pop() else {
            return Ok(());
        };

        apply_operations(&mut self.scene, &record.forward)?;
        self.undo_stack.push(record);
        Ok(())
    }

    /// Replay a sequence of transactions onto a fresh scene.
    pub fn replay(transactions: &[Transaction]) -> SceneResult<SceneDoc> {
        let mut manager = Self::new(SceneDoc::new());
        for transaction in transactions {
            manager.apply(transaction.clone())?;
        }
        Ok(manager.scene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_core::UlidGenerator;
    use c3d_scene_schema::{Name, Transform, TransformOp};

    #[test]
    fn undo_redo_transform() {
        let mut ids = UlidGenerator::new();
        let entity_id = ids.next_entity_id();

        let create = Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(Name::new("Camera")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
            }],
        );

        let move_tx = Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::TransformOp {
                entity_id,
                op: TransformOp::Translate(c3d_core::math::Vec3::new(1.0, 0.0, 0.0)),
            }],
        );

        let mut manager = TransactionManager::new(SceneDoc::new());
        manager.apply(create).expect("create entity");
        manager.apply(move_tx).expect("move entity");

        let moved = manager
            .scene()
            .get(entity_id)
            .expect("entity exists")
            .transform
            .translation;
        assert_eq!(moved.x, 1.0);

        manager.undo().expect("undo move");
        let original = manager
            .scene()
            .get(entity_id)
            .expect("entity exists")
            .transform
            .translation;
        assert_eq!(original, Transform::IDENTITY.translation);

        manager.redo().expect("redo move");
        let moved_again = manager
            .scene()
            .get(entity_id)
            .expect("entity exists")
            .transform
            .translation;
        assert_eq!(moved_again.x, 1.0);
    }
}

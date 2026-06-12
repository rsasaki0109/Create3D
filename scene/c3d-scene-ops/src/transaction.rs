use c3d_core::TransactionId;
use serde::{Deserialize, Serialize};

use crate::{SceneOperation, TransactionProvenance};

/// Ordered set of scene operations applied as one undo/redo step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Stable transaction identifier.
    pub id: TransactionId,
    /// Forward operations applied in order.
    pub operations: Vec<SceneOperation>,
    /// Optional AI or automation provenance metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<TransactionProvenance>,
}

impl Transaction {
    /// Create a new transaction.
    pub fn new(id: TransactionId, operations: Vec<SceneOperation>) -> Self {
        Self {
            id,
            operations,
            provenance: None,
        }
    }

    /// Create a transaction with provenance metadata.
    pub fn with_provenance(
        id: TransactionId,
        operations: Vec<SceneOperation>,
        provenance: TransactionProvenance,
    ) -> Self {
        Self {
            id,
            operations,
            provenance: Some(provenance),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_scene_schema::{Name, Transform};

    #[test]
    fn provenance_round_trips_through_serialization() {
        let transaction = Transaction::with_provenance(
            TransactionId::new(),
            vec![SceneOperation::CreateEntity {
                entity_id: c3d_core::EntityId::new(),
                parent: None,
                name: Some(Name::new("AI Entity")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
            }],
            TransactionProvenance {
                agent: "copilot".into(),
                user_prompt: "create entity Marker".into(),
                model_id: "mock-local".into(),
                tool_names: vec!["scene.create_entity".into()],
            },
        );

        let json = serde_json::to_string(&transaction).expect("serialize");
        let restored: Transaction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(transaction, restored);
    }
}

use c3d_core::TransactionId;
use serde::{Deserialize, Serialize};

use crate::SceneOperation;

/// Ordered set of scene operations applied as one undo/redo step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Stable transaction identifier.
    pub id: TransactionId,
    /// Forward operations applied in order.
    pub operations: Vec<SceneOperation>,
}

impl Transaction {
    /// Create a new transaction.
    pub fn new(id: TransactionId, operations: Vec<SceneOperation>) -> Self {
        Self { id, operations }
    }
}

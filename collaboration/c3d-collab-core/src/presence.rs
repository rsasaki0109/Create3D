use c3d_core::EntityId;
use serde::{Deserialize, Serialize};

use crate::ClientId;

/// Live presence information for a connected collaborator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPresence {
    /// Connected client identifier.
    pub client_id: ClientId,
    /// Display name.
    pub user_name: String,
    /// Currently selected entity, if any.
    pub selected_entity: Option<EntityId>,
    /// Viewport cursor position in normalized coordinates.
    pub cursor_viewport: Option<[f32; 2]>,
}

impl UserPresence {
    /// Create presence state for a newly connected user.
    pub fn new(client_id: ClientId, user_name: impl Into<String>) -> Self {
        Self {
            client_id,
            user_name: user_name.into(),
            selected_entity: None,
            cursor_viewport: None,
        }
    }
}

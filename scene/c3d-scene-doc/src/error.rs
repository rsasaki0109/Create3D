use c3d_core::EntityId;

/// Result alias for scene document operations.
pub type SceneResult<T> = Result<T, SceneError>;

/// Errors produced by `SceneDoc` mutations and serialization.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SceneError {
    /// Entity already exists.
    #[error("entity already exists: {0}")]
    EntityAlreadyExists(EntityId),

    /// Entity was not found.
    #[error("entity not found: {0}")]
    EntityNotFound(EntityId),

    /// Parent entity was not found.
    #[error("parent entity not found: {0}")]
    ParentNotFound(EntityId),

    /// Entity still has children and cannot be deleted.
    #[error("entity has children: {0}")]
    EntityHasChildren(EntityId),

    /// Attempted to make an entity its own parent.
    #[error("entity cannot parent itself: {0}")]
    SelfParent(EntityId),

    /// Scene schema validation failed.
    #[error("schema error: {0}")]
    Schema(#[from] c3d_scene_schema::SchemaError),

    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),
}

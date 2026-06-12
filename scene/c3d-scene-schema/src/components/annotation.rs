use serde::{Deserialize, Serialize};

/// Placeholder annotation component schema for Month 5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationPlaceholder {
    /// Free-form note text attached to an entity.
    pub text: String,
}

impl AnnotationPlaceholder {
    /// Create a placeholder annotation.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

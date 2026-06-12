use c3d_core::EntityId;

/// Tracks the currently selected scene entities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    primary: Option<EntityId>,
}

impl SelectionState {
    /// Create an empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the primary selected entity, if any.
    pub fn primary(&self) -> Option<EntityId> {
        self.primary
    }

    /// Select a single entity as the primary selection.
    pub fn select(&mut self, entity_id: EntityId) {
        self.primary = Some(entity_id);
    }

    /// Clear the current selection.
    pub fn clear(&mut self) {
        self.primary = None;
    }

    /// Returns true when the entity is currently selected.
    pub fn is_selected(&self, entity_id: EntityId) -> bool {
        self.primary == Some(entity_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_and_clear() {
        let mut selection = SelectionState::new();
        let entity = EntityId::new();
        selection.select(entity);
        assert!(selection.is_selected(entity));
        selection.clear();
        assert!(selection.primary().is_none());
    }
}

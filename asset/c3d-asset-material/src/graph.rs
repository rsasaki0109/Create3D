use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::MaterialAssetData;

/// Material graph evaluation failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MaterialGraphError {
    /// Generic graph error.
    #[error("invalid material graph: {0}")]
    Invalid(String),
}

/// Result alias for material graph operations.
pub type MaterialGraphResult<T> = Result<T, MaterialGraphError>;

/// Node kinds supported by the Month 6 material graph prototype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaterialGraphNodeKind {
    /// Constant RGBA parameter node.
    ConstantColor {
        /// Constant color value.
        color: [f32; 4],
    },
    /// Output node that exposes the final surface parameters.
    Output {
        /// Node id providing the base color value.
        base_color_node: u32,
    },
}

/// Material graph node with a stable local id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialGraphNode {
    /// Node identifier unique within the graph.
    pub id: u32,
    /// Node behavior.
    pub kind: MaterialGraphNodeKind,
}

/// Material graph data model stored alongside material assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialGraphData {
    /// Graph schema version.
    pub version: u32,
    /// Graph nodes.
    pub nodes: Vec<MaterialGraphNode>,
}

impl Default for MaterialGraphData {
    fn default() -> Self {
        Self::from_base_color([1.0, 1.0, 1.0, 1.0])
    }
}

impl MaterialGraphData {
    /// Create a simple single-color graph.
    pub fn from_base_color(color: [f32; 4]) -> Self {
        Self {
            version: 1,
            nodes: vec![
                MaterialGraphNode {
                    id: 0,
                    kind: MaterialGraphNodeKind::ConstantColor { color },
                },
                MaterialGraphNode {
                    id: 1,
                    kind: MaterialGraphNodeKind::Output { base_color_node: 0 },
                },
            ],
        }
    }

    /// Evaluate the graph into a render-ready material payload.
    pub fn evaluate(&self) -> MaterialGraphResult<MaterialAssetData> {
        let output = self
            .nodes
            .iter()
            .find(|node| matches!(node.kind, MaterialGraphNodeKind::Output { .. }))
            .ok_or_else(|| MaterialGraphError::Invalid("missing output node".into()))?;
        let MaterialGraphNodeKind::Output { base_color_node } = output.kind else {
            return Err(MaterialGraphError::Invalid("missing output node".into()));
        };
        let base_color = self.resolve_color(base_color_node)?;
        Ok(MaterialAssetData {
            version: 1,
            base_color,
            base_color_texture: None,
            graph: Some(self.clone()),
        })
    }

    fn resolve_color(&self, node_id: u32) -> MaterialGraphResult<[f32; 4]> {
        let node = self
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| MaterialGraphError::Invalid(format!("missing node {node_id}")))?;
        match node.kind {
            MaterialGraphNodeKind::ConstantColor { color } => Ok(color),
            MaterialGraphNodeKind::Output { .. } => Err(MaterialGraphError::Invalid(
                "output node cannot be used as color input".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_evaluates_base_color() {
        let graph = MaterialGraphData::from_base_color([0.2, 0.4, 0.8, 1.0]);
        let material = graph.evaluate().expect("evaluate graph");
        assert_eq!(material.base_color, [0.2, 0.4, 0.8, 1.0]);
    }
}

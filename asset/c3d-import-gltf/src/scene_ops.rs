use c3d_core::{EntityId, UlidGenerator};
use c3d_scene_ops::SceneOperation;
use c3d_scene_schema::{MaterialBinding, MeshRef, Name};

use crate::{GltfImportResult, ImportedNode};

/// Convert an import result into scene operations that create entities and bind assets.
pub fn import_result_to_scene_operations(
    import: &GltfImportResult,
    mesh_asset_ids: &[c3d_core::AssetId],
    material_asset_ids: &[c3d_core::AssetId],
    ids: &mut UlidGenerator,
) -> Vec<SceneOperation> {
    let mut operations = Vec::new();
    for node in &import.root_nodes {
        append_node_operations(
            node,
            None,
            mesh_asset_ids,
            material_asset_ids,
            import,
            ids,
            &mut operations,
        );
    }
    operations
}

fn append_node_operations(
    node: &ImportedNode,
    parent: Option<EntityId>,
    mesh_asset_ids: &[c3d_core::AssetId],
    material_asset_ids: &[c3d_core::AssetId],
    import: &GltfImportResult,
    ids: &mut UlidGenerator,
    operations: &mut Vec<SceneOperation>,
) {
    let entity_id = ids.next_entity_id();
    let mesh_ref = node.mesh_index.map(|index| {
        let mut mesh_ref = MeshRef::new(mesh_asset_ids[index]);
        mesh_ref.submesh = Some(import.meshes[index].name.clone());
        mesh_ref
    });
    let material_binding = node.mesh_index.and_then(|mesh_index| {
        import.meshes[mesh_index]
            .material_index
            .map(|material_index| MaterialBinding::new(material_asset_ids[material_index]))
    });

    operations.push(SceneOperation::CreateEntity {
        entity_id,
        parent,
        name: node.name.clone().map(Name::new),
        transform: node.transform,
        mesh_ref,
        material_binding,
        point_cloud_ref: None,
        gaussian_splat_ref: None,
    });

    for child in &node.children {
        append_node_operations(
            child,
            Some(entity_id),
            mesh_asset_ids,
            material_asset_ids,
            import,
            ids,
            operations,
        );
    }
}

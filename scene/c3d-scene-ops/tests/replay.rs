//! Integration tests for transaction replay.

use c3d_core::{EntityId, TransactionId, UlidGenerator};
use c3d_scene_doc::SceneDoc;
use c3d_scene_ops::{SceneOperation, Transaction, TransactionManager};
use c3d_scene_schema::{GaussianSplatRef, Name, Transform, TransformOp};

#[test]
fn replay_transactions_yields_identical_scene() {
    let mut ids = UlidGenerator::new();
    let root_id = ids.next_entity_id();
    let child_id = ids.next_entity_id();

    let transactions = vec![
        Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::CreateEntity {
                entity_id: root_id,
                parent: None,
                name: Some(Name::new("Root")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
            }],
        ),
        Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::CreateEntity {
                entity_id: child_id,
                parent: Some(root_id),
                name: Some(Name::new("Child")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: None,
            }],
        ),
        Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::TransformOp {
                entity_id: child_id,
                op: TransformOp::Translate(c3d_core::math::Vec3::new(0.0, 2.0, 0.0)),
            }],
        ),
    ];

    let mut manager = TransactionManager::new(SceneDoc::new());
    for tx in &transactions {
        manager.apply(tx.clone()).expect("apply transaction");
    }

    let replayed = TransactionManager::replay(&transactions).expect("replay transactions");
    assert_eq!(manager.scene(), &replayed);
}

#[test]
fn serialized_scene_matches_replayed_scene() {
    let entity_id = EntityId::new();

    let transactions = vec![Transaction::new(
        TransactionId::new(),
        vec![SceneOperation::CreateEntity {
            entity_id,
            parent: None,
            name: Some(Name::new("ReplayMe")),
            transform: Transform::IDENTITY,
            mesh_ref: None,
            material_binding: None,
            point_cloud_ref: None,
            gaussian_splat_ref: None,
        }],
    )];

    let replayed = TransactionManager::replay(&transactions).expect("replay");

    let json = replayed.to_json().expect("serialize replayed scene");
    let loaded = SceneDoc::from_json(&json).expect("load serialized scene");
    assert_eq!(replayed, loaded);
}

#[test]
fn gaussian_splat_ref_replays_and_serializes() {
    let mut ids = UlidGenerator::new();
    let entity_id = ids.next_entity_id();
    let asset_id = ids.next_asset_id();

    let transactions = vec![
        Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::CreateEntity {
                entity_id,
                parent: None,
                name: Some(Name::new("SplatRoot")),
                transform: Transform::IDENTITY,
                mesh_ref: None,
                material_binding: None,
                point_cloud_ref: None,
                gaussian_splat_ref: Some(GaussianSplatRef::new(asset_id)),
            }],
        ),
        Transaction::new(
            ids.next_transaction_id(),
            vec![SceneOperation::SetGaussianSplatRef {
                entity_id,
                gaussian_splat_ref: GaussianSplatRef {
                    asset_id,
                    opacity_scale: 0.5,
                    size_scale: 2.0,
                    crop_filter: None,
                },
            }],
        ),
    ];

    let replayed = TransactionManager::replay(&transactions).expect("replay");
    let entity = replayed.get(entity_id).expect("entity");
    let gaussian_splat_ref = entity
        .gaussian_splat_ref
        .as_ref()
        .expect("gaussian splat ref");
    assert_eq!(gaussian_splat_ref.opacity_scale, 0.5);
    assert_eq!(gaussian_splat_ref.size_scale, 2.0);

    let json = replayed.to_json().expect("serialize replayed scene");
    let loaded = SceneDoc::from_json(&json).expect("load serialized scene");
    assert_eq!(replayed, loaded);
}

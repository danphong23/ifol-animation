//! Final hardening: persistence, scene session lifecycle, and deterministic scale.

use ifol_engine::{
    ComponentRecord, EngineBuilder, EntityKey, OpaqueRecord, PackageId, PackageManifest,
    PackageResolver, SceneDocument, SceneId, SceneLoader, SchemaRegistry, Version,
};

#[test]
fn manifest_round_trip_preserves_escaped_project_values() {
    let manifest = ifol_engine::ProjectManifest::new("demo \"project\"", "scenes/main\nnext")
        .with_package(
            PackageId::new("pkg.demo").unwrap(),
            ifol_engine::VersionReq::caret(Version::new(1, 2, 3)),
        );
    let serialized = manifest.serialize();
    let parsed = ifol_engine::ProjectManifest::parse(&serialized).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn scene_replacement_is_load_first_and_clear_preserves_runtime_contract() {
    let mut engine = EngineBuilder::new().build().unwrap();
    let first = SceneId::new("scene.first").unwrap();
    let second = SceneId::new("scene.second").unwrap();

    let mut first_doc = SceneDocument::new();
    first_doc.create_entity(EntityKey(1));
    first_doc.create_entity(EntityKey(2));
    let first_result = engine.load_scene_as(first.clone(), &first_doc).unwrap();
    assert_eq!(first_result.scene_id, Some(first.clone()));
    assert_eq!(engine.active_scene(), Some(&first));
    assert_eq!(engine.active_scene_entity_count(), 2);

    let mut invalid = SceneDocument::new();
    invalid.entities.push(EntityKey(7));
    invalid.entities.push(EntityKey(7));
    assert!(engine.load_scene_as(second.clone(), &invalid).is_err());
    assert_eq!(engine.active_scene(), Some(&first));
    assert_eq!(engine.active_scene_entity_count(), 2);

    let mut second_doc = SceneDocument::new();
    second_doc.create_entity(EntityKey(9));
    engine.load_scene_as(second.clone(), &second_doc).unwrap();
    assert_eq!(engine.active_scene(), Some(&second));
    assert_eq!(engine.active_scene_entity_count(), 1);

    assert!(engine.clear_scene().unwrap());
    assert_eq!(engine.active_scene(), None);
    assert_eq!(engine.active_scene_entity_count(), 0);
    assert!(!engine.clear_scene().unwrap());
}

#[test]
fn explicit_opaque_scene_records_are_preserved_by_loader() {
    let mut doc = SceneDocument::new();
    doc.add_opaque(OpaqueRecord {
        entity_key: EntityKey(42),
        record: ComponentRecord {
            schema: "future.component".into(),
            version: 7,
            payload: vec![1, 2, 3, 4],
        },
    });
    let result = SceneLoader::load_scene(
        &mut ifol_ecs::world::World::new(),
        &doc,
        &SchemaRegistry::new(),
        &ifol_engine::MigrationRegistry::new(),
    )
    .unwrap();
    assert_eq!(result.preserved_opaque.len(), 1);
    assert_eq!(result.preserved_opaque[0].record.payload, vec![1, 2, 3, 4]);
}

#[test]
fn resolver_remains_deterministic_for_a_large_dependency_chain() {
    let mut resolver = PackageResolver::new();
    for index in (0..256).rev() {
        let id = PackageId::new(format!("pkg.{index:03}")).unwrap();
        let mut manifest = PackageManifest::new(id, Version::new(1, 0, 0));
        if index > 0 {
            manifest = manifest.with_dependency(ifol_engine::PackageDependency {
                package_id: PackageId::new(format!("pkg.{:03}", index - 1)).unwrap(),
                version_req: ifol_engine::VersionReq::caret(Version::new(1, 0, 0)),
            });
        }
        resolver.add(manifest);
    }
    let lock = resolver.resolve().unwrap();
    assert_eq!(lock.len(), 256);
    for (index, package) in lock.packages.iter().enumerate() {
        assert_eq!(package.id.as_str(), format!("pkg.{index:03}"));
    }
}

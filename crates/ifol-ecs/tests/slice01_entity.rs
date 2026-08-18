mod support;

use ifol_ecs::entity::{EntityId, EntityManager};
use ifol_ecs::error::EcsError;
use std::collections::HashSet;

#[test]
fn slice01_entity_lifecycle_and_generational_safety() {
    let mut mgr = EntityManager::new();

    // 1. Initial state
    assert_eq!(mgr.alive_count(), 1); // WORLD entity
    assert!(mgr.is_alive(EntityId::WORLD));

    // 2. Spawn 100 entities
    let mut entities = Vec::with_capacity(100);
    for _ in 0..100 {
        entities.push(mgr.spawn());
    }
    assert_eq!(mgr.alive_count(), 101);

    // 3. Despawn even index entities
    let mut despawned_indices = HashSet::new();
    for (idx, &e) in entities.iter().enumerate() {
        if idx % 2 == 0 {
            assert!(mgr.despawn(e).is_ok());
            despawned_indices.insert(e.index());
        }
    }
    assert_eq!(mgr.alive_count(), 51);

    // 4. Stale ID rejection
    for (idx, &e) in entities.iter().enumerate() {
        if idx % 2 == 0 {
            assert!(!mgr.is_alive(e));
            assert_eq!(mgr.despawn(e), Err(EcsError::EntityNotFound(e)));
        } else {
            assert!(mgr.is_alive(e));
        }
    }

    // 5. Forged ID rejection on free slots
    for &free_idx in &despawned_indices {
        let forged_next_gen = EntityId::new(free_idx, 2);
        assert!(!mgr.is_alive(forged_next_gen));
        assert_eq!(
            mgr.validate(forged_next_gen),
            Err(EcsError::ForgedEntityId(forged_next_gen))
        );
    }

    // 6. Recycled slot allocation increments generation
    let mut new_entities = Vec::new();
    for _ in 0..50 {
        new_entities.push(mgr.spawn());
    }
    assert_eq!(mgr.alive_count(), 101);

    for &new_e in &new_entities {
        assert!(despawned_indices.contains(&new_e.index()));
        assert_eq!(new_e.generation(), 2);
        assert!(mgr.is_alive(new_e));
    }

    // 7. Protection of WORLD entity
    assert_eq!(
        mgr.despawn(EntityId::WORLD),
        Err(EcsError::EntityNotFound(EntityId::WORLD))
    );
}

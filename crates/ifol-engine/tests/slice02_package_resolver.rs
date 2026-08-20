//! Slice 2 — Package Identity & Resolver
//!
//! Acceptance criteria (from docs/05-implementation-plan.md):
//! - duplicate, missing, incompatible version, cycle
//! - multiple candidates, input-order permutation
//! - platform capability (future, tested via error variant)
//! - all deterministic

use ifol_engine::{
    PackageDependency, PackageId, PackageManifest, PackageResolver, ResolveError, Version,
    VersionReq,
};

// ═══════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════

fn pkg(id: &str, ver: (u32, u32, u32)) -> PackageManifest {
    PackageManifest::new(
        PackageId::new(id).unwrap(),
        Version::new(ver.0, ver.1, ver.2),
    )
}

fn dep(id: &str, ver: (u32, u32, u32)) -> PackageDependency {
    PackageDependency {
        package_id: PackageId::new(id).unwrap(),
        version_req: VersionReq::caret(Version::new(ver.0, ver.1, ver.2)),
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. IDENTITY
// ═══════════════════════════════════════════════════════════════════

#[test]
fn package_id_valid_formats() {
    assert!(PackageId::new("simple").is_some());
    assert!(PackageId::new("with-dashes").is_some());
    assert!(PackageId::new("with_underscores").is_some());
    assert!(PackageId::new("com.domain.pkg").is_some());
}

#[test]
fn package_id_rejects_empty() {
    assert!(PackageId::new("").is_none());
}

#[test]
fn package_id_rejects_special_chars() {
    assert!(PackageId::new("has space").is_none());
    assert!(PackageId::new("path/sep").is_none());
    assert!(PackageId::new("@scope").is_none());
    assert!(PackageId::new("a:b").is_none());
}

// ═══════════════════════════════════════════════════════════════════
// 2. VERSION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn version_parse_round_trip() {
    let v = Version::parse("3.14.159").unwrap();
    assert_eq!(format!("{v}"), "3.14.159");
}

#[test]
fn version_comparison() {
    assert!(Version::new(1, 0, 0) < Version::new(1, 0, 1));
    assert!(Version::new(1, 0, 0) < Version::new(1, 1, 0));
    assert!(Version::new(1, 0, 0) < Version::new(2, 0, 0));
}

#[test]
fn caret_compatibility() {
    let req = VersionReq::caret(Version::new(1, 2, 0));
    assert!(req.matches(&Version::new(1, 2, 0)));
    assert!(req.matches(&Version::new(1, 9, 0)));
    assert!(!req.matches(&Version::new(2, 0, 0)));
    assert!(!req.matches(&Version::new(1, 1, 0)));
}

// ═══════════════════════════════════════════════════════════════════
// 3. RESOLVER — HAPPY PATH
// ═══════════════════════════════════════════════════════════════════

#[test]
fn resolve_empty() {
    let lock = PackageResolver::new().resolve().unwrap();
    assert!(lock.is_empty());
}

#[test]
fn resolve_single() {
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 0, 0)));
    let lock = r.resolve().unwrap();
    assert_eq!(lock.len(), 1);
}

#[test]
fn resolve_linear_chain() {
    // alpha <- beta <- gamma
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 0, 0)));
    r.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    r.add(pkg("gamma", (1, 0, 0)).with_dependency(dep("beta", (1, 0, 0))));
    let lock = r.resolve().unwrap();
    assert_eq!(lock.len(), 3);
    assert_eq!(lock.packages[0].id.as_str(), "alpha");
    assert_eq!(lock.packages[1].id.as_str(), "beta");
    assert_eq!(lock.packages[2].id.as_str(), "gamma");
}

#[test]
fn resolve_diamond() {
    let mut r = PackageResolver::new();
    r.add(pkg("core", (1, 0, 0)));
    r.add(pkg("left", (1, 0, 0)).with_dependency(dep("core", (1, 0, 0))));
    r.add(pkg("right", (1, 0, 0)).with_dependency(dep("core", (1, 0, 0))));
    r.add(
        pkg("top", (1, 0, 0))
            .with_dependency(dep("left", (1, 0, 0)))
            .with_dependency(dep("right", (1, 0, 0))),
    );
    let lock = r.resolve().unwrap();
    assert_eq!(lock.len(), 4);
    assert_eq!(lock.packages[0].id.as_str(), "core");
    assert_eq!(lock.packages[3].id.as_str(), "top");
}

// ═══════════════════════════════════════════════════════════════════
// 4. RESOLVER — ERRORS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn resolve_duplicate_id() {
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 0, 0)));
    r.add(pkg("alpha", (2, 0, 0)));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::DuplicateId(_)
    ));
}

#[test]
fn resolve_missing_dependency() {
    let mut r = PackageResolver::new();
    r.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::MissingDependency { .. }
    ));
}

#[test]
fn resolve_version_conflict() {
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 0, 0)));
    r.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (2, 0, 0))));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::VersionConflict { .. }
    ));
}

#[test]
fn resolve_self_cycle() {
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::CycleDetected(_)
    ));
}

#[test]
fn resolve_two_node_cycle() {
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 0, 0)).with_dependency(dep("beta", (1, 0, 0))));
    r.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::CycleDetected(_)
    ));
}

#[test]
fn resolve_three_node_cycle() {
    let mut r = PackageResolver::new();
    r.add(pkg("a", (1, 0, 0)).with_dependency(dep("b", (1, 0, 0))));
    r.add(pkg("b", (1, 0, 0)).with_dependency(dep("c", (1, 0, 0))));
    r.add(pkg("c", (1, 0, 0)).with_dependency(dep("a", (1, 0, 0))));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::CycleDetected(_)
    ));
}

// ═══════════════════════════════════════════════════════════════════
// 5. DETERMINISM
// ═══════════════════════════════════════════════════════════════════

#[test]
fn input_order_independence() {
    // Order 1: gamma, alpha, beta
    let mut r1 = PackageResolver::new();
    r1.add(pkg("gamma", (1, 0, 0)).with_dependency(dep("beta", (1, 0, 0))));
    r1.add(pkg("alpha", (1, 0, 0)));
    r1.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));

    // Order 2: alpha, beta, gamma
    let mut r2 = PackageResolver::new();
    r2.add(pkg("alpha", (1, 0, 0)));
    r2.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    r2.add(pkg("gamma", (1, 0, 0)).with_dependency(dep("beta", (1, 0, 0))));

    // Order 3: beta, gamma, alpha
    let mut r3 = PackageResolver::new();
    r3.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    r3.add(pkg("gamma", (1, 0, 0)).with_dependency(dep("beta", (1, 0, 0))));
    r3.add(pkg("alpha", (1, 0, 0)));

    let l1 = r1.resolve().unwrap();
    let l2 = r2.resolve().unwrap();
    let l3 = r3.resolve().unwrap();

    assert_eq!(l1, l2);
    assert_eq!(l2, l3);
}

#[test]
fn lock_lookup_by_id() {
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 2, 3)));
    r.add(pkg("beta", (4, 5, 6)));
    let lock = r.resolve().unwrap();
    assert!(lock.find(&PackageId::new("alpha").unwrap()).is_some());
    assert!(lock.find(&PackageId::new("beta").unwrap()).is_some());
    assert!(lock.find(&PackageId::new("missing").unwrap()).is_none());
}

// ═══════════════════════════════════════════════════════════════════
// 6. EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn many_independent_packages() {
    let mut r = PackageResolver::new();
    for i in 0..50 {
        r.add(pkg(&format!("pkg-{i:03}"), (1, 0, 0)));
    }
    let lock = r.resolve().unwrap();
    assert_eq!(lock.len(), 50);
    // Should be sorted alphabetically since no deps
    assert_eq!(lock.packages[0].id.as_str(), "pkg-000");
    assert_eq!(lock.packages[49].id.as_str(), "pkg-049");
}

#[test]
fn version_compatible_minor_bump() {
    // alpha 1.0.0, beta requires ^1.0.0 — alpha 1.5.0 should satisfy
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (1, 5, 0)));
    r.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    let lock = r.resolve().unwrap();
    assert_eq!(lock.len(), 2);
}

#[test]
fn version_incompatible_major_bump() {
    // alpha 2.0.0, beta requires ^1.0.0 — should fail
    let mut r = PackageResolver::new();
    r.add(pkg("alpha", (2, 0, 0)));
    r.add(pkg("beta", (1, 0, 0)).with_dependency(dep("alpha", (1, 0, 0))));
    assert!(matches!(
        r.resolve().unwrap_err(),
        ResolveError::VersionConflict { .. }
    ));
}

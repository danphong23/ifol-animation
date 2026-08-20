//! Slice 5 — Project Container & Namespace
//!
//! Acceptance criteria:
//! - Project manifest validation, serialize/parse roundtrip
//! - Storage abstraction and path containment security (prevent path traversal)
//! - Hierarchical namespace claiming and collision rejection
//! - Package lock file serialization and parsing
//! - ProjectContainer save and load roundtrip

use ifol_engine::{
    CURRENT_FORMAT_VERSION, MemoryStorage, Namespace, NamespaceError, NamespaceRegistry, PackageId,
    PackageLock, PackageLockFile, PathSecurity, ProjectContainer, ProjectManifest, ProjectStorage,
    ResolvedPackage, StorageError, Version, VersionReq,
};

// ═══════════════════════════════════════════════════════════════════
// 1. PATH SECURITY & STORAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn path_security_sanitizes_valid_relative_paths() {
    assert_eq!(
        PathSecurity::sanitize("assets/image.png").unwrap(),
        "assets/image.png"
    );
    assert_eq!(
        PathSecurity::sanitize("./scenes/main.ifol").unwrap(),
        "scenes/main.ifol"
    );
    assert_eq!(
        PathSecurity::sanitize("deep\\nested\\file.txt").unwrap(),
        "deep/nested/file.txt"
    );
}

#[test]
fn path_security_rejects_traversal() {
    assert!(matches!(
        PathSecurity::sanitize("../escape.txt"),
        Err(StorageError::InvalidPath(_))
    ));
    assert!(matches!(
        PathSecurity::sanitize("scenes/../../escape.txt"),
        Err(StorageError::InvalidPath(_))
    ));
    assert!(matches!(
        PathSecurity::sanitize(".."),
        Err(StorageError::InvalidPath(_))
    ));
}

#[test]
fn path_security_rejects_absolute_and_drive_paths() {
    assert!(matches!(
        PathSecurity::sanitize("/etc/passwd"),
        Err(StorageError::InvalidPath(_))
    ));
    assert!(matches!(
        PathSecurity::sanitize("\\Windows\\System32"),
        Err(StorageError::InvalidPath(_))
    ));
    assert!(matches!(
        PathSecurity::sanitize("C:\\projects\\secret.txt"),
        Err(StorageError::InvalidPath(_))
    ));
}

#[test]
fn memory_storage_read_write_list() {
    let mut storage = MemoryStorage::new();
    assert!(!storage.exists("data.bin"));

    storage.write_file("data.bin", b"hello world").unwrap();
    assert!(storage.exists("data.bin"));

    let data = storage.read_file("data.bin").unwrap();
    assert_eq!(data, b"hello world");

    let files = storage.list_files().unwrap();
    assert_eq!(files, vec!["data.bin"]);
}

// ═══════════════════════════════════════════════════════════════════
// 2. NAMESPACE REGISTRY & COLLISION PREVENTION
// ═══════════════════════════════════════════════════════════════════

#[test]
fn namespace_valid_and_invalid_formats() {
    assert!(Namespace::new("core").is_some());
    assert!(Namespace::new("core.render").is_some());
    assert!(Namespace::new("vendor.pkg_a.module-1").is_some());

    assert!(Namespace::new("").is_none());
    assert!(Namespace::new(".leading_dot").is_none());
    assert!(Namespace::new("trailing_dot.").is_none());
    assert!(Namespace::new("double..dot").is_none());
    assert!(Namespace::new("has space").is_none());
}

#[test]
fn namespace_claims_and_lookups() {
    let mut reg = NamespaceRegistry::new();
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_b = PackageId::new("pkg-b").unwrap();

    let ns_a = Namespace::new("pkg_a.render").unwrap();
    let ns_b = Namespace::new("pkg_b.physics").unwrap();

    reg.claim(pkg_a.clone(), ns_a.clone()).unwrap();
    reg.claim(pkg_b.clone(), ns_b.clone()).unwrap();

    assert_eq!(reg.get_owner(&ns_a), Some(&pkg_a));
    assert_eq!(reg.get_owner(&ns_b), Some(&pkg_b));
    assert_eq!(reg.len(), 2);
}

#[test]
fn duplicate_namespace_claim_rejected() {
    let mut reg = NamespaceRegistry::new();
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_b = PackageId::new("pkg-b").unwrap();

    let ns = Namespace::new("shared.namespace").unwrap();
    reg.claim(pkg_a, ns.clone()).unwrap();

    let err = reg.claim(pkg_b, ns).unwrap_err();
    assert!(matches!(err, NamespaceError::DuplicateClaim { .. }));
}

#[test]
fn prefix_conflict_rejected() {
    let mut reg = NamespaceRegistry::new();
    let pkg_a = PackageId::new("pkg-a").unwrap();
    let pkg_b = PackageId::new("pkg-b").unwrap();

    // 1. Parent claimed first, child rejected
    reg.claim(pkg_a.clone(), Namespace::new("core").unwrap())
        .unwrap();
    let err1 = reg
        .claim(pkg_b.clone(), Namespace::new("core.render").unwrap())
        .unwrap_err();
    assert!(matches!(err1, NamespaceError::PrefixConflict { .. }));

    // 2. Child claimed first, parent rejected
    let mut reg2 = NamespaceRegistry::new();
    reg2.claim(pkg_a, Namespace::new("media.video").unwrap())
        .unwrap();
    let err2 = reg2
        .claim(pkg_b, Namespace::new("media").unwrap())
        .unwrap_err();
    assert!(matches!(err2, NamespaceError::PrefixConflict { .. }));
}

// ═══════════════════════════════════════════════════════════════════
// 3. PROJECT MANIFEST SERIALIZATION & PARSING
// ═══════════════════════════════════════════════════════════════════

#[test]
fn project_manifest_roundtrip() {
    let manifest = ProjectManifest::new("Motion Reel 2026", "scenes/intro.ifol")
        .with_package(
            PackageId::new("core-render").unwrap(),
            VersionReq::caret(Version::new(1, 0, 0)),
        )
        .with_package(
            PackageId::new("audio-engine").unwrap(),
            VersionReq::caret(Version::new(2, 1, 0)),
        );

    let serialized = manifest.serialize();
    let parsed = ProjectManifest::parse(&serialized).unwrap();

    assert_eq!(parsed.format_version, CURRENT_FORMAT_VERSION);
    assert_eq!(parsed.name, "Motion Reel 2026");
    assert_eq!(parsed.entry_scene, "scenes/intro.ifol");
    assert_eq!(parsed.required_packages.len(), 2);
    assert_eq!(
        parsed.required_packages[0].package_id.as_str(),
        "core-render"
    );
    assert_eq!(
        parsed.required_packages[1].package_id.as_str(),
        "audio-engine"
    );
}

// ═══════════════════════════════════════════════════════════════════
// 4. PACKAGE LOCKFILE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn package_lockfile_roundtrip() {
    let lock = PackageLock {
        packages: vec![
            ResolvedPackage {
                id: PackageId::new("alpha").unwrap(),
                version: Version::new(1, 0, 0),
                dependencies: vec![],
            },
            ResolvedPackage {
                id: PackageId::new("beta").unwrap(),
                version: Version::new(1, 2, 0),
                dependencies: vec![PackageId::new("alpha").unwrap()],
            },
        ],
    };

    let lockfile = PackageLockFile::from_lock(&lock);
    let serialized = lockfile.serialize();
    let parsed = PackageLockFile::parse(&serialized).unwrap();

    assert_eq!(parsed.packages.len(), 2);
    assert_eq!(parsed.packages[0].id.as_str(), "alpha");
    assert_eq!(parsed.packages[1].id.as_str(), "beta");
    assert_eq!(parsed.packages[1].dependencies.len(), 1);
    assert_eq!(parsed.packages[1].dependencies[0].as_str(), "alpha");
}

// ═══════════════════════════════════════════════════════════════════
// 5. PROJECT CONTAINER SAVE & LOAD
// ═══════════════════════════════════════════════════════════════════

#[test]
fn project_container_save_and_load_roundtrip() {
    let mut container = ProjectContainer::new_memory("Showcase", "scenes/main.ifol");
    container.manifest = container.manifest.with_package(
        PackageId::new("pkg-shape").unwrap(),
        VersionReq::caret(Version::new(1, 0, 0)),
    );

    let lock = PackageLock {
        packages: vec![ResolvedPackage {
            id: PackageId::new("pkg-shape").unwrap(),
            version: Version::new(1, 0, 0),
            dependencies: vec![],
        }],
    };
    container.lockfile = Some(PackageLockFile::from_lock(&lock));

    // Save into internal storage
    container.save().unwrap();

    // Verify written files exist
    assert!(container.storage.exists("manifest.ifol"));
    assert!(container.storage.exists("package.lock"));

    // Reload from the same storage
    let loaded = ProjectContainer::load(container.storage).unwrap();
    assert_eq!(loaded.manifest.name, "Showcase");
    assert_eq!(loaded.manifest.entry_scene, "scenes/main.ifol");
    assert_eq!(loaded.manifest.required_packages.len(), 1);
    assert!(loaded.lockfile.is_some());
    assert_eq!(loaded.lockfile.unwrap().packages.len(), 1);
}

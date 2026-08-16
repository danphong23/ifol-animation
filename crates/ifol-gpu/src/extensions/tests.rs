use super::*;

struct TestExtension {
    descriptor: ExtensionDescriptor,
    usages: Vec<ResourceUsage>,
}

impl GpuExtension for TestExtension {
    fn descriptor(&self) -> ExtensionDescriptor {
        self.descriptor.clone()
    }
}

impl ExtensionOperation for TestExtension {
    fn resource_usages(&self) -> &[ResourceUsage] {
        &self.usages
    }

    fn validate_operation(&self) -> Result<(), ExtensionValidationError> {
        validate_resource_usages(&self.usages)
    }
}

impl ExtensionDispatcher for TestExtension {
    fn descriptor(&self) -> ExtensionDescriptor {
        self.descriptor.clone()
    }

    fn encode(
        &self,
        _context: ExtensionExecutionContext<'_, '_>,
    ) -> Result<(), ExtensionExecutionError> {
        Ok(())
    }
}

fn extension(id: &str, version: u32) -> Arc<dyn GpuExtension> {
    Arc::new(TestExtension {
        descriptor: ExtensionDescriptor::new(id, version).unwrap(),
        usages: Vec::new(),
    })
}

#[test]
fn registration_rejects_empty_id_and_duplicate() {
    assert_eq!(
        ExtensionId::new("  "),
        Err(ExtensionRegistrationError::EmptyId)
    );

    let mut registry = ExtensionRegistry::new();
    registry.register(extension("test.filter", 1)).unwrap();
    assert_eq!(
        registry.register(extension("test.filter", 2)),
        Err(ExtensionRegistrationError::Duplicate(
            ExtensionId::new("test.filter").unwrap()
        ))
    );
}

#[test]
fn registration_keeps_versioned_extension_discoverable() {
    let mut registry = ExtensionRegistry::new();
    registry.register(extension("test.compute", 7)).unwrap();
    let id = ExtensionId::new("test.compute").unwrap();

    assert_eq!(registry.len(), 1);
    assert!(registry.contains(&id));
    assert_eq!(registry.get(&id).unwrap().descriptor().version, 7);
}

#[test]
fn operation_contract_preserves_usage_and_rejects_invalid_ranges() {
    let valid = TestExtension {
        descriptor: ExtensionDescriptor::new("test.operation", 1).unwrap(),
        usages: vec![ResourceUsage {
            resource: GraphResource::Buffer(crate::render::BufferHandle(3)),
            access: ResourceAccess::ReadWrite,
            subresource: ResourceSubresource::BufferRange { start: 4, end: 12 },
        }],
    };
    assert_eq!(valid.resource_usages().len(), 1);
    assert_eq!(valid.validate_operation(), Ok(()));

    let invalid = [ResourceUsage {
        resource: GraphResource::Buffer(crate::render::BufferHandle(3)),
        access: ResourceAccess::Write,
        subresource: ResourceSubresource::BufferRange { start: 12, end: 12 },
    }];
    assert_eq!(
        validate_resource_usages(&invalid),
        Err(ExtensionValidationError::InvalidResourceRange)
    );
}

#[test]
fn dispatch_registry_rejects_duplicate_and_keeps_versioned_dispatcher() {
    let dispatcher = Arc::new(TestExtension {
        descriptor: ExtensionDescriptor::new("test.dispatch", 3).unwrap(),
        usages: Vec::new(),
    });
    let mut registry = ExtensionDispatchRegistry::new();
    registry.register(dispatcher.clone()).unwrap();
    assert_eq!(registry.len(), 1);
    let id = ExtensionId::new("test.dispatch").unwrap();
    assert!(registry.contains(&id));
    assert_eq!(registry.get(&id).unwrap().descriptor().version, 3);
    assert_eq!(
        registry.register(dispatcher),
        Err(ExtensionDispatchRegistrationError::Duplicate(id))
    );
}

#[test]
fn dispatcher_default_validation_reuses_resource_contract() {
    let dispatcher = TestExtension {
        descriptor: ExtensionDescriptor::new("test.validation", 1).unwrap(),
        usages: Vec::new(),
    };
    let invalid = [ResourceUsage {
        resource: GraphResource::Buffer(crate::render::BufferHandle(4)),
        access: ResourceAccess::Read,
        subresource: ResourceSubresource::BufferRange { start: 9, end: 9 },
    }];
    assert_eq!(
        dispatcher.validate(&invalid),
        Err(ExtensionValidationError::InvalidResourceRange)
    );
}

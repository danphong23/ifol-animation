use std::collections::HashMap;
use std::sync::Arc;

use super::{
    ExtensionDispatchRegistrationError, ExtensionDispatcher, ExtensionId,
    ExtensionRegistrationError, GpuExtension,
};

#[derive(Default)]
pub struct ExtensionRegistry {
    entries: HashMap<ExtensionId, Arc<dyn GpuExtension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        extension: Arc<dyn GpuExtension>,
    ) -> Result<(), ExtensionRegistrationError> {
        let descriptor = extension.descriptor();
        if self.entries.contains_key(&descriptor.id) {
            return Err(ExtensionRegistrationError::Duplicate(descriptor.id));
        }
        self.entries.insert(descriptor.id, extension);
        Ok(())
    }

    pub fn get(&self, id: &ExtensionId) -> Option<&Arc<dyn GpuExtension>> {
        self.entries.get(id)
    }

    pub fn contains(&self, id: &ExtensionId) -> bool {
        self.entries.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Default, Clone)]
pub struct ExtensionDispatchRegistry {
    entries: HashMap<ExtensionId, Arc<dyn ExtensionDispatcher>>,
}

impl ExtensionDispatchRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        dispatcher: Arc<dyn ExtensionDispatcher>,
    ) -> Result<(), ExtensionDispatchRegistrationError> {
        let id = dispatcher.descriptor().id;
        if self.entries.contains_key(&id) {
            return Err(ExtensionDispatchRegistrationError::Duplicate(id));
        }
        self.entries.insert(id, dispatcher);
        Ok(())
    }

    pub fn get(&self, id: &ExtensionId) -> Option<&Arc<dyn ExtensionDispatcher>> {
        self.entries.get(id)
    }

    pub fn contains(&self, id: &ExtensionId) -> bool {
        self.entries.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

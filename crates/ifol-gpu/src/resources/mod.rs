pub mod descriptors;
pub mod handle;
/// Internal implementation modules stay private; these modules are the
/// responsibility-oriented resource boundaries behind the public facade.
mod lookup;
mod mutation;
mod ownership;
mod registry;
mod versions;

pub use descriptors::*;
pub use handle::*;
pub use registry::{MeshResource, OwnedTextureResource, ResourceRegistry};

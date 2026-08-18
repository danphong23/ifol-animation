/// Marker trait for any data type that can be stored as a component in ECS.
///
/// Any type implementing `'static + Send + Sync` automatically implements `Component`.
pub trait Component: 'static + Send + Sync {}

impl<T: 'static + Send + Sync> Component for T {}

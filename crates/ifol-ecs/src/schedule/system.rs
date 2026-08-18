use crate::world::World;

/// Trait implemented by any executable system in `ifol-ecs`.
pub trait System: 'static + Send + Sync {
    /// Returns the diagnostic name of this system.
    fn name(&self) -> &str;

    /// Executes the system logic on the world.
    fn run(&mut self, world: &mut World);
}

/// A wrapper converting any matching closure or function pointer into a `System`.
pub struct FunctionSystem<F> {
    name: String,
    func: F,
}

impl<F> FunctionSystem<F> {
    /// Creates a new function system with a descriptive name.
    pub fn new<S: Into<String>>(name: S, func: F) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

impl<F> System for FunctionSystem<F>
where
    F: FnMut(&mut World) + 'static + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, world: &mut World) {
        (self.func)(world);
    }
}

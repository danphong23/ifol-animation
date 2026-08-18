use crate::error::SystemError;
use crate::system::SystemContext;

/// Trait implemented by all executable units in `ifol-ecs`.
///
/// Systems receive a controlled `SystemContext` and do not have raw access to `World`.
pub trait System: 'static + Send + Sync {
    /// Executes the system logic using the provided context.
    fn run(&mut self, ctx: &mut SystemContext<'_>) -> Result<(), SystemError>;
}

/// Function-based system adapter enabling closures to act as systems.
pub struct FunctionSystem<F> {
    f: F,
}

impl<F> FunctionSystem<F>
where
    F: FnMut(&mut SystemContext<'_>) -> Result<(), SystemError> + 'static + Send + Sync,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> System for FunctionSystem<F>
where
    F: FnMut(&mut SystemContext<'_>) -> Result<(), SystemError> + 'static + Send + Sync,
{
    fn run(&mut self, ctx: &mut SystemContext<'_>) -> Result<(), SystemError> {
        (self.f)(ctx)
    }
}

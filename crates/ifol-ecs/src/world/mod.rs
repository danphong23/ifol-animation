#[path = "world.rs"]
mod container;
mod singleton;

#[cfg(test)]
mod tests;

pub use container::World;

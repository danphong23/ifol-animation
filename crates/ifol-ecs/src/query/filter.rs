use crate::storage::Component;
use std::marker::PhantomData;

/// Query filter: only matches entities that have component `T`.
#[derive(Debug, Clone, Copy)]
pub struct With<T: Component>(PhantomData<T>);

/// Query filter: only matches entities that DO NOT have component `T`.
#[derive(Debug, Clone, Copy)]
pub struct Without<T: Component>(PhantomData<T>);

//! `ifol-gpu` là GPU execution core của iFol.
//!
//! Crate nhận resource, pipeline, shader contract và render graph từ host;
//! core validate, sắp xếp dependency/hazard, execute và hỗ trợ raw readback.
//! Core không decode asset, quản lý màu cấp sản phẩm hay encode PNG/JPEG/video.
//!
//! Người dùng bên ngoài nên bắt đầu từ `README.md` của crate và
//! `docs/60-guides/README.md`. Public API chính nằm trong các module
//! `api`, `backend`, `resources`, `graph`, `execution`, `memory` và
//! `extensions`.

pub mod api;
pub mod backend;
pub mod graph;
pub mod resources;
pub mod execution;
pub mod extensions;
pub mod memory;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

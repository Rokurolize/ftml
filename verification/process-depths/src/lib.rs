//! Exhaustive bounded check of `process_depths` (src/parsing/depth.rs).
//!
//! This crate compiles the real source files by path, not a copy or a model.
//! `tests/exhaustive.rs` runs every input sequence inside the bounds.

#![allow(dead_code)]

#[path = "../../../src/non_empty_vec.rs"]
pub mod non_empty_vec;

#[path = "../../../src/parsing/depth.rs"]
pub mod depth;

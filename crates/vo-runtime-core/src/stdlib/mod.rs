//! Standard library native function implementations.
//!
//! This module provides native implementations for Vo standard library functions.
//! 
//! Native functions are implemented using `#[vo_extern_std]` macro which:
//! - Validates signature against .vo file declaration
//! - Generates `#[no_mangle] extern "C"` wrapper for dynamic library export

pub mod fmt;

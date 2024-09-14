//!
//! Commonly used nodes for integrating NComm with other useful
//! tools.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[deny(missing_docs)]
#[cfg(feature = "rerun")]
pub mod rerun;
#[cfg(feature = "rerun")]
pub use rerun::RerunNode;

//!
//! Utilities for NComm
//!
//! The main usage of this crate is for traits and items
//! that don't really fit in any of the other NComm traits, but
//! are still useful for developing applications utilizing NComm.
//!

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod packing;

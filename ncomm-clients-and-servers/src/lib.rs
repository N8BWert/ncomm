//!
//! NComm Clients and Servers
//!
//! This crate contains a set of commonly used servers and their
//! respective clients to enable the sharing of data between
//! Nodes.
//!

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
pub mod local;

#[cfg(feature = "std")]
pub mod udp;

pub mod serial;

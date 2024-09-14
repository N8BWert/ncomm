//!
//! NComm Update Clients and Servers
//!
//! This crate contains a set of commonly used update clients and servers that
//! are created such that a request is a long-running task with updates and a
//! final response containing the requested information.
//!

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
pub mod local;

#[cfg(feature = "std")]
pub mod udp;

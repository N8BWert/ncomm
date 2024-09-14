//!
//! NComm Publishers and Subscribers
//!
//! This create contains a set of commonly used publishers and their
//! respective subscribers to enable sharing of data between Nodes
//! as effortless as choosing the correct publisher and subscriber.
//!

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
pub mod local;

#[cfg(feature = "std")]
pub mod udp;

pub mod tcp;

#[cfg(feature = "rerun")]
pub mod rerun;

pub mod serial;

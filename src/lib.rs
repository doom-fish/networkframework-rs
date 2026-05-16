#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API documentation

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod client;
pub mod error;
pub mod ffi;
pub mod listener;

pub use client::TcpClient;
pub use error::NetworkError;
pub use listener::TcpListener;

/// Common imports.
pub mod prelude {
    pub use crate::client::TcpClient;
    pub use crate::error::NetworkError;
    pub use crate::listener::TcpListener;
}

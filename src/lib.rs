#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API documentation

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod browser;
pub mod client;
pub mod error;
pub mod ffi;
pub mod listener;
pub mod path_monitor;
pub mod quic;
pub mod udp;
pub mod websocket;

pub use browser::{
    advertise_bonjour_service, start_browser, BonjourAdvertiser, Browser, BrowserEvent,
    DiscoveredService,
};
pub use client::TcpClient;
pub use error::NetworkError;
pub use listener::TcpListener;
pub use path_monitor::{start_path_monitor, InterfaceType, PathMonitor, PathUpdate};
pub use quic::QuicConnection;
pub use udp::UdpClient;
pub use websocket::{Opcode, WebSocket, WsMessage};

/// Common imports.
pub mod prelude {
    pub use crate::browser::{
        advertise_bonjour_service, start_browser, BonjourAdvertiser, Browser, BrowserEvent,
        DiscoveredService,
    };
    pub use crate::client::TcpClient;
    pub use crate::error::NetworkError;
    pub use crate::listener::TcpListener;
    pub use crate::path_monitor::{
        start_path_monitor, InterfaceType, PathMonitor, PathUpdate,
    };
    pub use crate::quic::QuicConnection;
    pub use crate::udp::UdpClient;
    pub use crate::websocket::{Opcode, WebSocket, WsMessage};
}

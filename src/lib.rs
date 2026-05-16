#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API documentation

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod browser;
pub mod client;
pub mod connection_group;
pub mod error;
pub mod ffi;
pub mod framer;
pub mod interface;
pub mod listener;
pub mod parameters;
pub mod path_monitor;
pub mod privacy;
pub mod quic;
pub mod udp;
pub mod websocket;

pub use browser::{
    advertise_bonjour_service, start_browser, BonjourAdvertiser, Browser, BrowserEvent,
    DiscoveredService,
};
pub use client::{ContentContext, ReceivedContent, TcpClient};
pub use connection_group::{
    ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupMessage, ConnectionGroupState,
};
pub use error::NetworkError;
pub use framer::{
    Framer, FramerContext, FramerDefinition, FramerMessage, FramerMessageView, FramerOptions,
    FramerStart,
};
pub use interface::{list_interfaces, InterfaceType, NetworkInterface};
pub use listener::TcpListener;
pub use parameters::ConnectionParameters;
pub use path_monitor::{start_path_monitor, PathMonitor, PathUpdate};
pub use privacy::{PrivacyContext, ProxyConfig, ResolverConfig};
pub use quic::QuicConnection;
pub use udp::UdpClient;
pub use websocket::{Opcode, WebSocket, WsMessage};

/// Common imports.
pub mod prelude {
    pub use crate::browser::{
        advertise_bonjour_service, start_browser, BonjourAdvertiser, Browser, BrowserEvent,
        DiscoveredService,
    };
    pub use crate::client::{ContentContext, ReceivedContent, TcpClient};
    pub use crate::connection_group::{
        ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupMessage, ConnectionGroupState,
    };
    pub use crate::error::NetworkError;
    pub use crate::framer::{
        Framer, FramerContext, FramerDefinition, FramerMessage, FramerMessageView,
        FramerOptions, FramerStart,
    };
    pub use crate::interface::{list_interfaces, InterfaceType, NetworkInterface};
    pub use crate::listener::TcpListener;
    pub use crate::parameters::ConnectionParameters;
    pub use crate::path_monitor::{start_path_monitor, PathMonitor, PathUpdate};
    pub use crate::privacy::{PrivacyContext, ProxyConfig, ResolverConfig};
    pub use crate::quic::QuicConnection;
    pub use crate::udp::UdpClient;
    pub use crate::websocket::{Opcode, WebSocket, WsMessage};
}

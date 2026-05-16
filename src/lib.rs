#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API documentation

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod advertise_descriptor;
pub mod browser;
pub mod client;
pub mod connection;
pub mod connection_group;
pub mod content_context;
pub mod endpoint;
pub mod error;
pub mod ffi;
pub mod framer;
pub mod group;
pub mod interface;
pub mod listener;
pub mod parameters;
pub mod path;
pub mod path_monitor;
pub mod privacy;
pub mod privacy_context;
pub mod protocol;
pub mod proxy_config;
pub mod quic;
#[cfg(feature = "raw-ffi")]
pub mod raw_ffi;
pub mod resolver;
pub mod udp;
pub mod websocket;

pub use advertise_descriptor::{advertise_with_descriptor, AdvertiseDescriptor, Advertiser};
pub use browser::{
    advertise_bonjour_service, start_browser, start_browser_with_descriptor, BonjourAdvertiser,
    BrowseDescriptor, Browser, BrowserEvent, DiscoveredService,
};
pub use client::{ContentContext, ReceivedContent, TcpClient};
pub use connection::Connection;
pub use connection_group::{
    ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupMessage, ConnectionGroupState,
};
pub use endpoint::{Endpoint, EndpointType};
pub use error::NetworkError;
pub use framer::{
    Framer, FramerContext, FramerDefinition, FramerMessage, FramerMessageView, FramerOptions,
    FramerStart,
};
pub use group::{Group, GroupDescriptor, GroupMessage, GroupState};
pub use interface::{list_interfaces, InterfaceType, NetworkInterface};
pub use listener::TcpListener;
pub use parameters::{ConnectionParameters, ParametersAttribution};
pub use path::{LinkQuality, Path, PathStatus, PathUnsatisfiedReason};
pub use path_monitor::{start_path_monitor, PathMonitor, PathUpdate};
pub use privacy::{PrivacyContext, ProxyConfig, RelayHop, ResolverConfig};
pub use protocol::{ProtocolDefinition, ProtocolOptions};
pub use quic::{QuicConnection, QuicOptions};
pub use udp::UdpClient;
pub use websocket::{Opcode, WebSocket, WsMessage};

/// Common imports.
pub mod prelude {
    pub use crate::advertise_descriptor::{
        advertise_with_descriptor, AdvertiseDescriptor, Advertiser,
    };
    pub use crate::browser::{
        advertise_bonjour_service, start_browser, start_browser_with_descriptor, BonjourAdvertiser,
        BrowseDescriptor, Browser, BrowserEvent, DiscoveredService,
    };
    pub use crate::client::{ContentContext, ReceivedContent, TcpClient};
    pub use crate::connection::Connection;
    pub use crate::connection_group::{
        ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupMessage, ConnectionGroupState,
    };
    pub use crate::endpoint::{Endpoint, EndpointType};
    pub use crate::error::NetworkError;
    pub use crate::framer::{
        Framer, FramerContext, FramerDefinition, FramerMessage, FramerMessageView, FramerOptions,
        FramerStart,
    };
    pub use crate::group::{Group, GroupDescriptor, GroupMessage, GroupState};
    pub use crate::interface::{list_interfaces, InterfaceType, NetworkInterface};
    pub use crate::listener::TcpListener;
    pub use crate::parameters::{ConnectionParameters, ParametersAttribution};
    pub use crate::path::{LinkQuality, Path, PathStatus, PathUnsatisfiedReason};
    pub use crate::path_monitor::{start_path_monitor, PathMonitor, PathUpdate};
    pub use crate::privacy::{PrivacyContext, ProxyConfig, RelayHop, ResolverConfig};
    pub use crate::protocol::{ProtocolDefinition, ProtocolOptions};
    pub use crate::quic::{QuicConnection, QuicOptions};
    pub use crate::udp::UdpClient;
    pub use crate::websocket::{Opcode, WebSocket, WsMessage};
}

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
pub mod connection_report;
pub mod content_context;
pub mod endpoint;
mod endpoint_support;
pub mod error;
pub mod ethernet_channel;
pub mod ffi;
pub mod framer;
pub mod group;
pub mod interface;
mod interface_support;
pub mod listener;
pub mod parameters;
mod parameters_support;
pub mod path;
pub mod path_monitor;
pub mod privacy;
pub mod privacy_context;
pub mod protocol;
pub mod proxy_config;
pub mod quic;
mod quic_support;
#[cfg(feature = "raw-ffi")]
pub mod raw_ffi;
pub mod resolver;
pub mod txt_record;
pub mod udp;
pub mod websocket;

pub use advertise_descriptor::{advertise_with_descriptor, AdvertiseDescriptor, Advertiser};
pub use browser::{
    advertise_bonjour_service, start_browser, start_browser_results_with_descriptor,
    start_browser_with_descriptor, BonjourAdvertiser, BrowseDescriptor, BrowseResult,
    BrowseResultChange, BrowseResultsBrowser, Browser, BrowserEvent, BrowserState,
    DiscoveredService,
};
pub use client::{ContentContext, ReceivedContent, TcpClient};
pub use connection::Connection;
pub use connection_group::{
    ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupMessage, ConnectionGroupState,
};
pub use connection_report::{
    DataTransferPathReport, DataTransferReport, DataTransferReportState, EstablishmentProtocol,
    EstablishmentReport, ResolutionProtocol, ResolutionReport, ResolutionSource, ResolutionStep,
};
pub use endpoint::{Endpoint, EndpointType};
pub use error::{ErrorDomain, FrameworkError, NetworkError};
pub use ethernet_channel::{EthernetChannel, EthernetChannelState, EthernetFrame};
pub use framer::{
    Framer, FramerContext, FramerDefinition, FramerMessage, FramerMessageView, FramerOptions,
    FramerStart,
};
pub use group::{Group, GroupDescriptor, GroupMessage, GroupState};
pub use interface::{list_interfaces, InterfaceType, NetworkInterface};
pub use interface_support::InterfaceRadioType;
pub use listener::TcpListener;
pub use parameters::{ConnectionParameters, ParametersAttribution};
pub use parameters_support::{ExpiredDnsBehavior, MultipathService, ProtocolStack, ServiceClass};
pub use path::{LinkQuality, Path, PathStatus, PathUnsatisfiedReason};
pub use path_monitor::{
    start_path_monitor, start_path_monitor_for_ethernet_channel, start_path_monitor_with_type,
    PathMonitor, PathUpdate,
};
pub use privacy::{PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, UrlSessionConfiguration};
pub use protocol::{
    IpEcnFlag, IpLocalAddressPreference, IpVersion, ProtocolDefinition, ProtocolMetadata,
    ProtocolOptions, TcpMultipathVersion,
};
pub use quic::{QuicConnection, QuicOptions};
pub use quic_support::{
    QuicMetadata, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions,
};
pub use txt_record::{TxtRecord, TxtRecordEntry, TxtRecordFindResult, TxtRecordLookup};
pub use udp::UdpClient;
pub use websocket::{
    Opcode, WebSocket, WsCloseCode, WsMessage, WsRequest, WsResponse, WsResponseStatus,
    WsVersion,
};

/// Common imports.
pub mod prelude {
    pub use crate::advertise_descriptor::{
        advertise_with_descriptor, AdvertiseDescriptor, Advertiser,
    };
    pub use crate::browser::{
        advertise_bonjour_service, start_browser, start_browser_results_with_descriptor,
        start_browser_with_descriptor, BonjourAdvertiser, BrowseDescriptor, BrowseResult,
        BrowseResultChange, BrowseResultsBrowser, Browser, BrowserEvent, BrowserState,
        DiscoveredService,
    };
    pub use crate::client::{ContentContext, ReceivedContent, TcpClient};
    pub use crate::connection::Connection;
    pub use crate::connection_group::{
        ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupMessage, ConnectionGroupState,
    };
    pub use crate::connection_report::{
        DataTransferPathReport, DataTransferReport, DataTransferReportState,
        EstablishmentProtocol, EstablishmentReport, ResolutionProtocol, ResolutionReport,
        ResolutionSource, ResolutionStep,
    };
    pub use crate::endpoint::{Endpoint, EndpointType};
    pub use crate::error::{ErrorDomain, FrameworkError, NetworkError};
    pub use crate::ethernet_channel::{EthernetChannel, EthernetChannelState, EthernetFrame};
    pub use crate::framer::{
        Framer, FramerContext, FramerDefinition, FramerMessage, FramerMessageView, FramerOptions,
        FramerStart,
    };
    pub use crate::group::{Group, GroupDescriptor, GroupMessage, GroupState};
    pub use crate::interface::{list_interfaces, InterfaceType, NetworkInterface};
    pub use crate::interface_support::InterfaceRadioType;
    pub use crate::listener::TcpListener;
    pub use crate::parameters::{ConnectionParameters, ParametersAttribution};
    pub use crate::parameters_support::{
        ExpiredDnsBehavior, MultipathService, ProtocolStack, ServiceClass,
    };
    pub use crate::path::{LinkQuality, Path, PathStatus, PathUnsatisfiedReason};
    pub use crate::path_monitor::{
        start_path_monitor, start_path_monitor_for_ethernet_channel, start_path_monitor_with_type,
        PathMonitor, PathUpdate,
    };
    pub use crate::privacy::{
        PrivacyContext, ProxyConfig, RelayHop, ResolverConfig, UrlSessionConfiguration,
    };
    pub use crate::protocol::{
        IpEcnFlag, IpLocalAddressPreference, IpVersion, ProtocolDefinition, ProtocolMetadata,
        ProtocolOptions, TcpMultipathVersion,
    };
    pub use crate::quic::{QuicConnection, QuicOptions};
    pub use crate::quic_support::{
        QuicMetadata, QuicStreamType, SecurityProtocolMetadata, SecurityProtocolOptions,
    };
    pub use crate::txt_record::{TxtRecord, TxtRecordEntry, TxtRecordFindResult, TxtRecordLookup};
    pub use crate::udp::UdpClient;
    pub use crate::websocket::{
        Opcode, WebSocket, WsCloseCode, WsMessage, WsRequest, WsResponse, WsResponseStatus,
        WsVersion,
    };
}

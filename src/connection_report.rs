//! Connection establishment and data-transfer reports.

#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]

use core::ffi::c_void;

use crate::{
    client::TcpClient,
    endpoint::Endpoint,
    error::{from_status, NetworkError},
    ffi,
    interface::NetworkInterface,
    interface_support::{network_interface_from_handle, InterfaceRadioType},
    protocol::ProtocolDefinition,
    quic::QuicConnection,
};

/// DNS resolution source used during connection establishment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSource {
    Query,
    Cache,
    ExpiredCache,
    Unknown(i32),
}

impl ResolutionSource {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Query,
            2 => Self::Cache,
            3 => Self::ExpiredCache,
            other => Self::Unknown(other),
        }
    }
}

/// Protocol used for a resolution step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionProtocol {
    Unknown,
    Udp,
    Tcp,
    Tls,
    Https,
    Other(i32),
}

impl ResolutionProtocol {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::Udp,
            2 => Self::Tcp,
            3 => Self::Tls,
            4 => Self::Https,
            other => Self::Other(other),
        }
    }
}

/// Current lifecycle state of a data-transfer report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTransferReportState {
    Collecting,
    Collected,
    Unknown(i32),
}

impl DataTransferReportState {
    const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Collecting,
            2 => Self::Collected,
            other => Self::Unknown(other),
        }
    }
}

/// Protocol handshake entry from an establishment report.
#[derive(Debug)]
pub struct EstablishmentProtocol {
    pub protocol_definition: ProtocolDefinition,
    pub handshake_milliseconds: u64,
    pub handshake_rtt_milliseconds: u64,
}

/// One direct resolution step from an establishment report.
#[derive(Debug)]
pub struct ResolutionStep {
    pub source: ResolutionSource,
    pub milliseconds: u64,
    pub endpoint_count: u32,
    pub successful_endpoint: Option<Endpoint>,
    pub preferred_endpoint: Option<Endpoint>,
}

/// One path entry from a collected data-transfer report.
#[derive(Debug, Clone)]
pub struct DataTransferPathReport {
    pub interface: Option<NetworkInterface>,
    pub radio_type: InterfaceRadioType,
    pub received_ip_packet_count: u64,
    pub sent_ip_packet_count: u64,
    pub received_transport_byte_count: u64,
    pub received_transport_duplicate_byte_count: u64,
    pub received_transport_out_of_order_byte_count: u64,
    pub sent_transport_byte_count: u64,
    pub sent_transport_retransmitted_byte_count: u64,
    pub transport_smoothed_rtt_milliseconds: u64,
    pub transport_minimum_rtt_milliseconds: u64,
    pub transport_rtt_variance: u64,
    pub received_application_byte_count: u64,
    pub sent_application_byte_count: u64,
}

/// Wrapper around `nw_resolution_report_t`.
pub struct ResolutionReport {
    handle: *mut c_void,
}

unsafe impl Send for ResolutionReport {}
unsafe impl Sync for ResolutionReport {}

impl ResolutionReport {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Source used for this resolution report.
    #[must_use]
    pub fn source(&self) -> ResolutionSource {
        ResolutionSource::from_raw(unsafe { ffi::nw_shim_resolution_report_get_source(self.handle) })
    }

    /// Duration of this resolution report in milliseconds.
    #[must_use]
    pub fn milliseconds(&self) -> u64 {
        unsafe { ffi::nw_shim_resolution_report_get_milliseconds(self.handle) }
    }

    /// Number of candidate endpoints considered by this step.
    #[must_use]
    pub fn endpoint_count(&self) -> u32 {
        unsafe { ffi::nw_shim_resolution_report_get_endpoint_count(self.handle) }
    }

    /// Successful endpoint, if one exists.
    #[must_use]
    pub fn successful_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_resolution_report_copy_successful_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Preferred endpoint, if one exists.
    #[must_use]
    pub fn preferred_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_resolution_report_copy_preferred_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Resolution protocol used by this step.
    #[must_use]
    pub fn protocol(&self) -> ResolutionProtocol {
        ResolutionProtocol::from_raw(unsafe { ffi::nw_shim_resolution_report_get_protocol(self.handle) })
    }
}

impl Clone for ResolutionReport {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for ResolutionReport {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Wrapper around `nw_establishment_report_t`.
pub struct EstablishmentReport {
    handle: *mut c_void,
}

unsafe impl Send for EstablishmentReport {}
unsafe impl Sync for EstablishmentReport {}

impl EstablishmentReport {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Total establishment time in milliseconds.
    #[must_use]
    pub fn duration_milliseconds(&self) -> u64 {
        unsafe { ffi::nw_shim_establishment_report_get_duration_milliseconds(self.handle) }
    }

    /// Delay between `start()` and the successful attempt.
    #[must_use]
    pub fn attempt_started_after_milliseconds(&self) -> u64 {
        unsafe {
            ffi::nw_shim_establishment_report_get_attempt_started_after_milliseconds(self.handle)
        }
    }

    /// Number of previous attempts before the successful one.
    #[must_use]
    pub fn previous_attempt_count(&self) -> u32 {
        unsafe { ffi::nw_shim_establishment_report_get_previous_attempt_count(self.handle) }
    }

    /// Whether a proxy was used.
    #[must_use]
    pub fn used_proxy(&self) -> bool {
        unsafe { ffi::nw_shim_establishment_report_get_used_proxy(self.handle) != 0 }
    }

    /// Whether any proxy configuration applied.
    #[must_use]
    pub fn proxy_configured(&self) -> bool {
        unsafe { ffi::nw_shim_establishment_report_get_proxy_configured(self.handle) != 0 }
    }

    /// Proxy endpoint used for the connection, if any.
    #[must_use]
    pub fn proxy_endpoint(&self) -> Option<Endpoint> {
        let handle = unsafe { ffi::nw_shim_establishment_report_copy_proxy_endpoint(self.handle) };
        (!handle.is_null()).then_some(unsafe { Endpoint::from_raw(handle) })
    }

    /// Protocol handshakes that occurred during establishment.
    #[must_use]
    pub fn protocols(&self) -> Vec<EstablishmentProtocol> {
        let mut count = 0_usize;
        let items = unsafe { ffi::nw_shim_establishment_report_copy_protocols(self.handle, &mut count) };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let protocols = slice
            .iter()
            .filter_map(|item| {
                (!item.protocol_definition.is_null()).then_some(EstablishmentProtocol {
                    protocol_definition: unsafe { ProtocolDefinition::from_raw(item.protocol_definition) },
                    handshake_milliseconds: item.handshake_milliseconds,
                    handshake_rtt_milliseconds: item.handshake_rtt_milliseconds,
                })
            })
            .collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        protocols
    }

    /// Direct resolution steps observed during establishment.
    #[must_use]
    pub fn resolutions(&self) -> Vec<ResolutionStep> {
        let mut count = 0_usize;
        let items = unsafe { ffi::nw_shim_establishment_report_copy_resolutions(self.handle, &mut count) };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let resolutions = slice
            .iter()
            .map(|item| ResolutionStep {
                source: ResolutionSource::from_raw(item.source),
                milliseconds: item.milliseconds,
                endpoint_count: item.endpoint_count,
                successful_endpoint: (!item.successful_endpoint.is_null())
                    .then_some(unsafe { Endpoint::from_raw(item.successful_endpoint) }),
                preferred_endpoint: (!item.preferred_endpoint.is_null())
                    .then_some(unsafe { Endpoint::from_raw(item.preferred_endpoint) }),
            })
            .collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        resolutions
    }

    /// Lower-level resolution reports observed during establishment.
    #[must_use]
    pub fn resolution_reports(&self) -> Vec<ResolutionReport> {
        let mut count = 0_usize;
        let items = unsafe {
            ffi::nw_shim_establishment_report_copy_resolution_reports(self.handle, &mut count)
        };
        if items.is_null() || count == 0 {
            return Vec::new();
        }
        let slice = unsafe { std::slice::from_raw_parts(items, count) };
        let reports = slice
            .iter()
            .filter_map(|handle| (!handle.is_null()).then_some(unsafe { ResolutionReport::from_raw(*handle) }))
            .collect();
        unsafe { ffi::nw_shim_free_buffer(items.cast()) };
        reports
    }
}

impl Clone for EstablishmentReport {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for EstablishmentReport {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

/// Wrapper around `nw_data_transfer_report_t`.
pub struct DataTransferReport {
    handle: *mut c_void,
}

unsafe impl Send for DataTransferReport {}
unsafe impl Sync for DataTransferReport {}

impl DataTransferReport {
    #[must_use]
    pub(crate) const unsafe fn from_raw(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /// Bitmask covering all collected paths.
    #[must_use]
    pub fn all_paths() -> u32 {
        unsafe { ffi::nw_shim_data_transfer_report_all_paths() }
    }

    /// Complete collection of the report.
    pub fn collect(&self) -> Result<(), NetworkError> {
        let status = unsafe { ffi::nw_shim_data_transfer_report_collect(self.handle) };
        if status != ffi::NW_OK {
            return Err(from_status(status));
        }
        Ok(())
    }

    /// Current collection state.
    #[must_use]
    pub fn state(&self) -> DataTransferReportState {
        DataTransferReportState::from_raw(unsafe { ffi::nw_shim_data_transfer_report_get_state(self.handle) })
    }

    /// Duration covered by the report, in milliseconds.
    #[must_use]
    pub fn duration_milliseconds(&self) -> u64 {
        unsafe { ffi::nw_shim_data_transfer_report_get_duration_milliseconds(self.handle) }
    }

    /// Number of path entries captured by the report.
    #[must_use]
    pub fn path_count(&self) -> u32 {
        unsafe { ffi::nw_shim_data_transfer_report_get_path_count(self.handle) }
    }

    /// Copy one path entry from the report.
    #[must_use]
    pub fn path(&self, path_index: u32) -> Option<DataTransferPathReport> {
        if path_index >= self.path_count() {
            return None;
        }
        let interface = unsafe {
            network_interface_from_handle(ffi::nw_shim_data_transfer_report_copy_path_interface(
                self.handle,
                path_index,
            ))
        };
        Some(DataTransferPathReport {
            interface,
            radio_type: InterfaceRadioType::from_raw(unsafe {
                ffi::nw_shim_data_transfer_report_get_path_radio_type(self.handle, path_index)
            }),
            received_ip_packet_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_received_ip_packet_count(self.handle, path_index)
            },
            sent_ip_packet_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_sent_ip_packet_count(self.handle, path_index)
            },
            received_transport_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_received_transport_byte_count(self.handle, path_index)
            },
            received_transport_duplicate_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_received_transport_duplicate_byte_count(
                    self.handle,
                    path_index,
                )
            },
            received_transport_out_of_order_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_received_transport_out_of_order_byte_count(
                    self.handle,
                    path_index,
                )
            },
            sent_transport_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_sent_transport_byte_count(self.handle, path_index)
            },
            sent_transport_retransmitted_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_sent_transport_retransmitted_byte_count(
                    self.handle,
                    path_index,
                )
            },
            transport_smoothed_rtt_milliseconds: unsafe {
                ffi::nw_shim_data_transfer_report_get_transport_smoothed_rtt_milliseconds(
                    self.handle,
                    path_index,
                )
            },
            transport_minimum_rtt_milliseconds: unsafe {
                ffi::nw_shim_data_transfer_report_get_transport_minimum_rtt_milliseconds(
                    self.handle,
                    path_index,
                )
            },
            transport_rtt_variance: unsafe {
                ffi::nw_shim_data_transfer_report_get_transport_rtt_variance(self.handle, path_index)
            },
            received_application_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_received_application_byte_count(
                    self.handle,
                    path_index,
                )
            },
            sent_application_byte_count: unsafe {
                ffi::nw_shim_data_transfer_report_get_sent_application_byte_count(self.handle, path_index)
            },
        })
    }

    /// Copy every path entry from the report.
    #[must_use]
    pub fn paths(&self) -> Vec<DataTransferPathReport> {
        (0..self.path_count())
            .filter_map(|index| self.path(index))
            .collect()
    }
}

impl Clone for DataTransferReport {
    fn clone(&self) -> Self {
        let handle = unsafe { ffi::nw_shim_retain_object(self.handle) };
        Self { handle }
    }
}

impl Drop for DataTransferReport {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::nw_shim_release_object(self.handle) };
            self.handle = core::ptr::null_mut();
        }
    }
}

impl TcpClient {
    /// Copy the connection establishment report, if the connection is ready.
    #[must_use]
    pub fn establishment_report(&self) -> Option<EstablishmentReport> {
        let handle = unsafe { ffi::nw_shim_connection_copy_establishment_report(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { EstablishmentReport::from_raw(handle) })
    }

    /// Create a new data-transfer report for this connection.
    #[must_use]
    pub fn data_transfer_report(&self) -> Option<DataTransferReport> {
        let handle = unsafe { ffi::nw_shim_connection_create_data_transfer_report(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { DataTransferReport::from_raw(handle) })
    }
}

impl QuicConnection {
    /// Copy the QUIC connection's establishment report, if the connection is ready.
    #[must_use]
    pub fn establishment_report(&self) -> Option<EstablishmentReport> {
        let handle = unsafe { ffi::nw_shim_connection_copy_establishment_report(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { EstablishmentReport::from_raw(handle) })
    }

    /// Create a new data-transfer report for this QUIC connection.
    #[must_use]
    pub fn data_transfer_report(&self) -> Option<DataTransferReport> {
        let handle = unsafe { ffi::nw_shim_connection_create_data_transfer_report(self.as_ptr()) };
        (!handle.is_null()).then_some(unsafe { DataTransferReport::from_raw(handle) })
    }
}

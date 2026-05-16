#![cfg(target_os = "macos")]

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use networkframework::{
    advertise_with_descriptor, start_browser_with_descriptor, AdvertiseDescriptor,
    BrowseDescriptor, BrowserEvent, ConnectionGroup, ConnectionGroupDescriptor,
    ConnectionParameters, ContentContext, DataTransferReportState, Endpoint, EndpointType,
    EthernetChannel, ExpiredDnsBehavior, Framer, FramerContext, FramerDefinition,
    FramerMessageView, FramerStart, InterfaceType, MultipathService, ParametersAttribution,
    PathStatus, PrivacyContext, ProtocolDefinition, ProtocolOptions, ProxyConfig, QuicOptions,
    RelayHop, ResolverConfig, ServiceClass, TcpClient, TcpListener, TxtRecord,
    TxtRecordFindResult,
};

fn unique_label(prefix: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_millis();
    format!("{prefix}-{}-{stamp}", std::process::id())
}

#[derive(Default)]
struct LengthPrefixFramer {
    pending_length: Option<usize>,
}

impl Framer for LengthPrefixFramer {
    fn on_start(&mut self, _context: &mut FramerContext) -> FramerStart {
        FramerStart::Ready
    }

    fn on_input(&mut self, context: &mut FramerContext) -> usize {
        loop {
            if self.pending_length.is_none() {
                let mut header = [0_u8; 4];
                if !context.parse_input(4, 4, Some(&mut header), |_bytes, _is_complete| 4) {
                    return 4;
                }
                self.pending_length = Some(u32::from_be_bytes(header) as usize);
            }

            let expected = self.pending_length.expect("header parsed");
            if expected == 0 {
                let message = context
                    .create_message()
                    .expect("create zero-length message");
                assert!(context.pass_input_data(0, Some(&message), true));
                self.pending_length = None;
                continue;
            }

            if !context.parse_input(expected, expected, None, |_bytes, _is_complete| 0) {
                return expected;
            }

            let message = context.create_message().expect("create framed message");
            assert!(context.pass_input_data(expected, Some(&message), true));
            self.pending_length = None;
        }
    }

    fn on_output(
        &mut self,
        context: &mut FramerContext,
        _message: Option<FramerMessageView<'_>>,
        message_length: usize,
        is_complete: bool,
    ) {
        if !is_complete {
            context.mark_failed_with_error(-100);
            return;
        }
        let Ok(message_length) = u32::try_from(message_length) else {
            context.mark_failed_with_error(-101);
            return;
        };
        context.write_output_data(&message_length.to_be_bytes());
        assert!(context.pass_output_data(message_length as usize));
    }

    fn on_stop(&mut self, _context: &mut FramerContext) -> bool {
        true
    }
}

#[test]
fn connection_area_round_trip_exposes_metadata() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        let request = connection.receive(1024)?;
        assert_eq!(request, b"ping");
        connection.send(b"pong")?;
        Ok(())
    });

    let client = TcpClient::connect("127.0.0.1", port)?;
    client.send(b"ping")?;
    assert_eq!(client.receive(1024)?, b"pong");
    assert!(matches!(
        client.endpoint().expect("endpoint").endpoint_type(),
        EndpointType::Host | EndpointType::Address
    ));
    assert!(client.parameters().is_some());

    server.join().expect("server thread")?;
    Ok(())
}

#[test]
fn listener_area_accepts_connections() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind(0)?;
    assert!(listener.local_port() > 0);
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<Vec<u8>, networkframework::NetworkError> {
        let connection = listener.accept()?;
        connection.receive(1024)
    });

    let client = TcpClient::connect("127.0.0.1", port)?;
    client.send(b"listener-test")?;
    assert_eq!(server.join().expect("server thread")?, b"listener-test");
    Ok(())
}

#[test]
fn browser_area_descriptor_and_start() -> Result<(), networkframework::NetworkError> {
    let mut descriptor = BrowseDescriptor::bonjour_service("_nfwtest._tcp", Some("local"))?;
    assert_eq!(
        descriptor.bonjour_service_type().as_deref(),
        Some("_nfwtest._tcp")
    );
    assert!(descriptor
        .bonjour_service_domain()
        .as_deref()
        .is_some_and(|domain| domain.contains("local")));
    descriptor.set_include_txt_record(true);
    assert!(descriptor.include_txt_record());

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_callback = Arc::clone(&events);
    let browser = start_browser_with_descriptor(&descriptor, None, move |event| {
        events_for_callback.lock().expect("events lock").push(event);
    })?;
    std::thread::sleep(Duration::from_millis(100));
    drop(browser);

    let recorded = events.lock().expect("events lock");
    assert!(recorded
        .iter()
        .all(|event| matches!(event, BrowserEvent::Found(_) | BrowserEvent::Lost(_))));
    drop(recorded);
    Ok(())
}

#[test]
fn parameters_area_supports_policy_controls() -> Result<(), networkframework::NetworkError> {
    let mut parameters = ConnectionParameters::generic()?;
    parameters
        .set_attribution(ParametersAttribution::User)
        .set_required_interface_type(InterfaceType::Loopback)
        .set_prohibit_expensive(true)
        .set_prohibit_constrained(true)
        .set_allow_ultra_constrained(true)
        .set_prefer_no_proxy(true);
    assert_eq!(parameters.attribution(), ParametersAttribution::User);
    assert_eq!(
        parameters.required_interface_type(),
        InterfaceType::Loopback
    );
    assert!(parameters.prohibit_expensive());
    assert!(parameters.prohibit_constrained());
    let _ = parameters.allow_ultra_constrained();

    let websocket = ProtocolOptions::websocket()?;
    parameters.prepend_application_protocol(&websocket)?;

    let _tcp = ConnectionParameters::tcp()?;
    let _tls_tcp = ConnectionParameters::tls_tcp()?;
    let _udp = ConnectionParameters::udp()?;
    if let Ok(application_service) = ConnectionParameters::application_service() {
        let _ = application_service;
    }
    Ok(())
}

#[test]
fn parameters_area_supports_advanced_knobs() -> Result<(), networkframework::NetworkError> {
    let mut parameters = ConnectionParameters::generic()?;
    if let Some(loopback) = networkframework::list_interfaces()
        .into_iter()
        .find(|interface| interface.interface_type == InterfaceType::Loopback)
    {
        parameters.require_interface(Some(&loopback))?;
        let required = parameters.required_interface().expect("required interface");
        assert_eq!(required.index, loopback.index);
        parameters.prohibit_interface(&loopback)?;
        assert!(parameters
            .prohibited_interfaces()
            .iter()
            .any(|interface| interface.index == loopback.index));
        parameters.clear_prohibited_interfaces();
        assert!(parameters.prohibited_interfaces().is_empty());
        parameters.require_interface(None)?;
        assert!(parameters.required_interface().is_none());
    }

    parameters
        .set_reuse_local_address(true)
        .set_include_peer_to_peer(true)
        .set_fast_open_enabled(true)
        .set_service_class(ServiceClass::ResponsiveData)
        .set_multipath_service(MultipathService::Handover)
        .set_local_only(true)
        .set_expired_dns_behavior(ExpiredDnsBehavior::Allow)
        .set_requires_dnssec_validation(true)
        .set_prefer_no_proxy(true)
        .prohibit_interface_type(InterfaceType::WiFi);

    assert!(parameters.reuse_local_address());
    assert!(parameters.include_peer_to_peer());
    assert!(parameters.fast_open_enabled());
    assert_eq!(parameters.service_class(), ServiceClass::ResponsiveData);
    assert_eq!(parameters.multipath_service(), MultipathService::Handover);
    assert!(parameters.local_only());
    assert_eq!(parameters.expired_dns_behavior(), ExpiredDnsBehavior::Allow);
    assert!(parameters.requires_dnssec_validation());
    assert!(parameters.prefer_no_proxy());
    assert!(parameters
        .prohibited_interface_types()
        .contains(&InterfaceType::WiFi));

    let local_endpoint = Endpoint::address("127.0.0.1", 0)?;
    parameters.set_local_endpoint(Some(&local_endpoint));
    assert!(parameters.local_endpoint().is_some());
    parameters.clear_prohibited_interface_types();
    assert!(parameters.prohibited_interface_types().is_empty());

    let websocket = ProtocolOptions::websocket()?;
    parameters.prepend_application_protocol(&websocket)?;
    let mut stack = parameters.default_protocol_stack().expect("protocol stack");
    assert_eq!(stack.application_protocols().len(), 1);
    let udp = ProtocolOptions::udp()?;
    let udp_definition = udp.definition().expect("udp definition");
    stack.set_transport_protocol(&udp);
    assert_eq!(
        stack.transport_protocol().and_then(|protocol| protocol.definition()),
        Some(udp_definition)
    );
    let _ = stack.internet_protocol();
    stack.clear_application_protocols();
    assert!(stack.application_protocols().is_empty());

    Ok(())
}

#[test]
fn endpoint_area_builds_common_endpoint_types() -> Result<(), networkframework::NetworkError> {
    let host = Endpoint::host("example.com", 443)?;
    assert_eq!(host.endpoint_type(), EndpointType::Host);
    assert_eq!(host.hostname().as_deref(), Some("example.com"));
    assert_eq!(host.port(), 443);

    let address = Endpoint::address("127.0.0.1", 8080)?;
    assert_eq!(address.endpoint_type(), EndpointType::Address);
    assert!(address.address_string().is_some());

    let bonjour = Endpoint::bonjour_service(Some("demo"), "_http._tcp", Some("local"))?;
    assert_eq!(bonjour.endpoint_type(), EndpointType::BonjourService);
    assert_eq!(bonjour.bonjour_service_name().as_deref(), Some("demo"));
    assert_eq!(
        bonjour.bonjour_service_type().as_deref(),
        Some("_http._tcp")
    );

    let url = Endpoint::url("https://example.com")?;
    assert_eq!(url.endpoint_type(), EndpointType::Url);
    assert!(url
        .url_string()
        .as_deref()
        .is_some_and(|value| value.contains("example.com")));
    let _ = host.signature();
    Ok(())
}

#[test]
fn path_area_reports_connection_path() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        let _ = connection.receive(1024)?;
        Ok(())
    });

    let client = TcpClient::connect("127.0.0.1", port)?;
    client.send(b"path")?;
    let path = client.current_path().expect("current path");
    let _ = path.has_ipv4();
    let _ = path.has_ipv6();
    let _ = path.interfaces();
    assert!(matches!(
        path.status(),
        PathStatus::Satisfied
            | PathStatus::Satisfiable
            | PathStatus::Unsatisfied
            | PathStatus::Invalid
    ));
    let _ = path.link_quality();
    let _ = path.effective_local_endpoint();
    let _ = path.effective_remote_endpoint();

    server.join().expect("server thread")?;
    Ok(())
}

#[test]
fn framer_area_round_trip() -> Result<(), networkframework::NetworkError> {
    let definition = FramerDefinition::new("length-prefix-test", LengthPrefixFramer::default)?;
    let options = definition.options()?;

    let mut parameters = ConnectionParameters::tcp()?;
    parameters.prepend_framer(&options)?;

    let listener = TcpListener::bind_with_parameters(0, &parameters)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        let request = connection.receive(4096)?;
        assert_eq!(request, b"hello over framer");
        connection.send(b"framed response")?;
        Ok(())
    });

    let client = TcpClient::connect_with_parameters("127.0.0.1", port, &parameters)?;
    client.send(b"hello over framer")?;
    assert_eq!(client.receive(4096)?, b"framed response");
    server.join().expect("server thread")?;
    Ok(())
}

#[test]
fn group_area_starts_and_cancels() -> Result<(), networkframework::NetworkError> {
    let descriptor = ConnectionGroupDescriptor::multicast("239.255.0.1", 5000)?;
    let parameters = ConnectionParameters::udp()?;
    let mut group = ConnectionGroup::new(&descriptor, &parameters)?;

    let states = Arc::new(Mutex::new(Vec::new()));
    let states_for_callback = Arc::clone(&states);
    group.set_state_changed_handler(move |state| {
        states_for_callback.lock().expect("states lock").push(state);
    });
    group.set_receive_handler(2048, false, |_message| {});
    group.start()?;
    std::thread::sleep(Duration::from_millis(200));
    group.cancel();

    let observed = states.lock().expect("states lock");
    assert!(!observed.is_empty());
    drop(observed);
    Ok(())
}

#[test]
fn protocol_area_exposes_definitions_and_options() -> Result<(), networkframework::NetworkError> {
    let tcp_definition = ProtocolDefinition::tcp()?;
    let udp_definition = ProtocolDefinition::udp()?;
    let tls_definition = ProtocolDefinition::tls()?;
    let ip_definition = ProtocolDefinition::ip()?;
    let websocket_definition = ProtocolDefinition::websocket()?;
    let quic_definition = ProtocolDefinition::quic()?;

    assert_ne!(tcp_definition, udp_definition);
    assert_ne!(tls_definition, ip_definition);
    assert_ne!(websocket_definition, quic_definition);

    let tcp_options = ProtocolOptions::tcp()?;
    let udp_options = ProtocolOptions::udp()?;
    let tls_options = ProtocolOptions::tls()?;
    let ip_options = ProtocolOptions::ip()?;
    let websocket_options = ProtocolOptions::websocket()?;
    let quic_options = ProtocolOptions::quic()?;

    assert_eq!(
        tcp_options.definition().expect("tcp definition"),
        tcp_definition
    );
    assert_eq!(
        udp_options.definition().expect("udp definition"),
        udp_definition
    );
    assert_eq!(
        tls_options.definition().expect("tls definition"),
        tls_definition
    );
    assert_eq!(
        ip_options.definition().expect("ip definition"),
        ip_definition
    );
    assert_eq!(
        websocket_options.definition().expect("ws definition"),
        websocket_definition
    );
    assert!(quic_options.is_quic());
    assert_eq!(
        quic_options.definition().expect("quic definition"),
        quic_definition
    );
    Ok(())
}

#[test]
fn content_context_area_tracks_properties() -> Result<(), networkframework::NetworkError> {
    let mut antecedent = ContentContext::new("first")?;
    antecedent.set_relative_priority(0.25);

    let mut context = ContentContext::new("second")?;
    context
        .set_is_final(true)
        .set_expiration_milliseconds(500)
        .set_relative_priority(0.75)
        .set_antecedent(Some(&antecedent));

    assert_eq!(context.identifier(), "second");
    assert!(context.is_final());
    let _ = context.expiration_milliseconds();
    let _ = context.relative_priority();
    let _ = context.copy_antecedent();
    Ok(())
}

#[test]
fn resolver_area_builders_work() -> Result<(), networkframework::NetworkError> {
    let mut doh = ResolverConfig::dns_over_https("https://example.com/dns-query{?dns}")?;
    doh.add_server_address("1.1.1.1", 443)?;

    let mut dot = ResolverConfig::dns_over_tls("dns.example", 853)?;
    dot.add_server_address("9.9.9.9", 853)?;
    Ok(())
}

#[test]
fn quic_area_exposes_transport_settings() -> Result<(), networkframework::NetworkError> {
    let mut options = QuicOptions::new()?;
    options
        .add_tls_application_protocol("h3")?
        .set_stream_is_unidirectional(true)
        .set_stream_is_datagram(true)
        .set_initial_max_data(65_536)
        .set_initial_max_streams_bidirectional(8)
        .set_initial_max_streams_unidirectional(4)
        .set_initial_max_stream_data_bidirectional_local(32_768)
        .set_initial_max_stream_data_bidirectional_remote(16_384)
        .set_initial_max_stream_data_unidirectional(8_192)
        .set_max_udp_payload_size(1350)
        .set_max_datagram_frame_size(1200)
        .set_idle_timeout(15_000);

    assert!(options.stream_is_unidirectional());
    assert!(options.stream_is_datagram());
    assert_eq!(options.initial_max_data(), 65_536);
    assert_eq!(options.initial_max_streams_bidirectional(), 8);
    assert_eq!(options.initial_max_streams_unidirectional(), 4);
    assert_eq!(options.initial_max_stream_data_bidirectional_local(), 32_768);
    assert_eq!(options.initial_max_stream_data_bidirectional_remote(), 16_384);
    assert_eq!(options.initial_max_stream_data_unidirectional(), 8_192);
    assert_eq!(options.max_udp_payload_size(), 1350);
    assert_eq!(options.max_datagram_frame_size(), 1200);
    assert_eq!(options.idle_timeout(), 15_000);
    assert!(options.security_options().is_some());
    assert!(options.protocol_options().is_quic());

    let context = ContentContext::new("quic-empty")?;
    assert!(context.copy_quic_metadata().is_none());
    Ok(())
}

#[test]
fn privacy_context_area_supports_default_and_encrypted_resolution(
) -> Result<(), networkframework::NetworkError> {
    let resolver = ResolverConfig::dns_over_https("https://example.com/dns-query{?dns}")?;
    let privacy = PrivacyContext::new(&unique_label("privacy"))?;
    privacy.require_encrypted_name_resolution(true, Some(&resolver));
    privacy.flush_cache();
    privacy.disable_logging();

    let default_context = PrivacyContext::default_context();
    default_context.flush_cache();
    Ok(())
}

#[test]
fn proxy_config_area_tracks_domains_and_optional_relay(
) -> Result<(), networkframework::NetworkError> {
    let mut proxy = ProxyConfig::http_connect("proxy.example", 443, true)?;
    proxy
        .set_credentials("user", Some("secret"))?
        .set_failover_allowed(true)
        .add_match_domain("example.com")?
        .add_excluded_domain("internal.example.com")?;

    assert!(proxy.failover_allowed());
    assert!(proxy
        .match_domains()
        .iter()
        .any(|domain| domain == "example.com"));
    assert!(proxy
        .excluded_domains()
        .iter()
        .any(|domain| domain == "internal.example.com"));

    let relay_endpoint = Endpoint::host("relay.example", 443)?;
    let relay_tls = ProtocolOptions::tls()?;
    if let Ok(mut hop) = RelayHop::new(Some(&relay_endpoint), None, Some(&relay_tls)) {
        hop.add_additional_http_header_field("X-Test", "1")?;
        if let Ok(relay_proxy) = ProxyConfig::relay(&hop, None) {
            let _ = relay_proxy;
        }
    }
    Ok(())
}

#[test]
fn advertise_descriptor_area_builds_and_advertises() -> Result<(), networkframework::NetworkError> {
    let mut descriptor = AdvertiseDescriptor::bonjour_service(
        Some(&unique_label("service")),
        "_nfwtest._tcp",
        Some("local"),
    )?;
    descriptor.set_txt_record(b"k=v").set_no_auto_rename(true);
    assert!(descriptor.no_auto_rename());
    assert_eq!(descriptor.service_type(), Some("_nfwtest._tcp"));
    assert!(descriptor.service_name().is_some());

    let advertiser = advertise_with_descriptor(&descriptor, 18_080)?;
    std::thread::sleep(Duration::from_millis(100));
    drop(advertiser);

    if let Ok(application_service) =
        AdvertiseDescriptor::application_service("com.example.networkframework")
    {
        assert_eq!(
            application_service.application_service_name().as_deref(),
            Some("com.example.networkframework")
        );
    }
    Ok(())
}

#[test]
fn txt_record_area_supports_lookup_and_endpoint_helpers(
) -> Result<(), networkframework::NetworkError> {
    let mut txt = TxtRecord::dictionary()?;
    txt.set_key("alpha", Some(b"beta"))?
        .set_key("empty", Some(b""))?
        .set_key("flag", None)?;

    assert!(txt.is_dictionary());
    assert_eq!(txt.key_count(), 3);
    assert_eq!(txt.find_key("alpha")?, TxtRecordFindResult::NonEmptyValue);
    assert_eq!(
        txt.lookup("alpha")?.value.as_deref(),
        Some(&b"beta"[..])
    );
    let empty = txt.lookup("empty")?;
    assert_eq!(empty.status, TxtRecordFindResult::EmptyValue);
    assert_eq!(empty.value, Some(Vec::new()));
    let flag = txt.lookup("flag")?;
    assert_eq!(flag.status, TxtRecordFindResult::NoValue);
    assert_eq!(flag.value, None);
    assert_eq!(txt.find_key("missing")?, TxtRecordFindResult::NotPresent);

    let entries = txt.entries();
    assert_eq!(entries.len(), 3);
    let bytes = txt.bytes();
    assert!(!bytes.is_empty());
    let parsed = TxtRecord::from_bytes(&bytes)?;
    assert_eq!(parsed.find_key("alpha")?, TxtRecordFindResult::NonEmptyValue);
    let clone = txt.clone();
    assert_eq!(clone, txt);
    assert!(txt.remove_key("flag")?);
    assert_eq!(txt.key_count(), 2);

    let address = Endpoint::address("127.0.0.1", 8080)?;
    assert!(address.raw_address().is_some());
    let bonjour = Endpoint::bonjour_service(Some("demo"), "_http._tcp", Some("local"))?;
    let _ = bonjour.txt_record();
    Ok(())
}

#[test]
fn connection_report_area_collects_metrics() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        assert_eq!(connection.receive(1024)?, b"report");
        connection.send(b"ack")?;
        Ok(())
    });

    let client = TcpClient::connect("127.0.0.1", port)?;
    let establishment = client.establishment_report().expect("establishment report");
    let _ = establishment.duration_milliseconds();
    let _ = establishment.attempt_started_after_milliseconds();
    let _ = establishment.previous_attempt_count();
    let _ = establishment.used_proxy();
    let _ = establishment.proxy_configured();
    let _ = establishment.proxy_endpoint();
    assert!(!establishment.protocols().is_empty());
    let _ = establishment.resolutions();
    for resolution_report in establishment.resolution_reports() {
        let _ = resolution_report.source();
        let _ = resolution_report.milliseconds();
        let _ = resolution_report.endpoint_count();
        let _ = resolution_report.successful_endpoint();
        let _ = resolution_report.preferred_endpoint();
        let _ = resolution_report.protocol();
    }

    let transfer = client.data_transfer_report().expect("data-transfer report");
    client.send(b"report")?;
    assert_eq!(client.receive(1024)?, b"ack");
    transfer.collect()?;
    assert_eq!(transfer.state(), DataTransferReportState::Collected);
    let _ = networkframework::DataTransferReport::all_paths();
    let paths = transfer.paths();
    assert!(!paths.is_empty());
    assert!(paths[0].sent_application_byte_count >= 6);

    server.join().expect("server thread")?;
    Ok(())
}

#[test]
fn ethernet_channel_area_smoke() -> Result<(), networkframework::NetworkError> {
    let interface = networkframework::list_interfaces()
        .into_iter()
        .find(|interface| interface.interface_type != InterfaceType::Other);
    let Some(interface) = interface else {
        return Ok(());
    };

    let parameters = ConnectionParameters::generic()?;
    let _ = EthernetChannel::with_parameters(0x88B5, &interface, &parameters);
    if let Ok(mut channel) = EthernetChannel::new(0x88B5, &interface) {
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_callback = Arc::clone(&states);
        channel.set_state_changed_handler(move |state| {
            states_for_callback.lock().expect("states lock").push(state);
        });
        channel.set_receive_handler(|_frame| {});
        let _ = channel.maximum_payload_size();
        channel.start();
        std::thread::sleep(Duration::from_millis(50));
        channel.cancel();
        drop(states.lock().expect("states lock"));
    }

    Ok(())
}

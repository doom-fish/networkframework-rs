#![cfg(target_os = "macos")]

use std::time::{SystemTime, UNIX_EPOCH};

use networkframework::{
    BrowseDescriptor, BrowseResultChange, Connection, ConnectionParameters, ContentContext,
    Endpoint, EndpointType, Framer, FramerContext, FramerDefinition, FramerMessageView,
    FramerStart, InterfaceType, NetworkError, ProtocolDefinition, ProtocolOptions, ServiceClass,
    TcpListener,
};

fn unique_label(prefix: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_millis();
    format!("{prefix}-{}-{stamp}", std::process::id())
}

#[derive(Default)]
struct MetadataFramer;

impl Framer for MetadataFramer {
    fn on_start(&mut self, _context: &mut FramerContext) -> FramerStart {
        FramerStart::Ready
    }

    fn on_input(&mut self, context: &mut FramerContext) -> usize {
        context.pass_through_input();
        0
    }

    fn on_output(
        &mut self,
        context: &mut FramerContext,
        _message: Option<FramerMessageView<'_>>,
        _message_length: usize,
        _is_complete: bool,
    ) {
        context.pass_through_output();
    }

    fn on_stop(&mut self, _context: &mut FramerContext) -> bool {
        true
    }
}

#[test]
fn connection_area_connect_with_parameters_tracks_snapshots() -> Result<(), NetworkError> {
    let listener = TcpListener::bind(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<Vec<u8>, NetworkError> {
        let connection = listener.accept()?;
        connection.receive(1024)
    });

    let mut parameters = ConnectionParameters::tcp()?;
    let local_endpoint = Endpoint::address("127.0.0.1", 0)?;
    parameters
        .set_required_interface_type(InterfaceType::Loopback)
        .set_local_endpoint(Some(&local_endpoint))
        .set_reuse_local_address(true);

    let client = Connection::connect_with_parameters("127.0.0.1", port, &parameters)?;
    let snapshot = client.parameters().expect("parameters snapshot");
    assert_eq!(snapshot.required_interface_type(), InterfaceType::Loopback);
    assert!(snapshot.reuse_local_address());
    assert!(snapshot.local_endpoint().is_some());

    let endpoint = client.endpoint().expect("remote endpoint");
    assert_eq!(endpoint.port(), port);
    client.send(b"connection-params")?;
    assert_eq!(server.join().expect("server thread")?, b"connection-params");
    Ok(())
}

#[test]
fn listener_area_bind_with_parameters_updates_connection_limit() -> Result<(), NetworkError> {
    let mut parameters = ConnectionParameters::tcp()?;
    parameters.set_reuse_local_address(true);

    let mut listener = TcpListener::bind_with_parameters(0, &parameters)?;
    assert!(listener.local_port() > 0);
    let updated_limit = listener.new_connection_limit().max(1).saturating_add(1);
    listener.set_new_connection_limit(updated_limit);
    assert_eq!(listener.new_connection_limit(), updated_limit);

    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<u16, NetworkError> {
        let connection = listener.accept()?;
        let payload = connection.receive(1024)?;
        assert_eq!(payload, b"listener-limit");
        Ok(connection.endpoint().expect("accepted endpoint").port())
    });

    let client = Connection::connect("127.0.0.1", port)?;
    client.send(b"listener-limit")?;
    assert!(server.join().expect("server thread")? > 0);
    Ok(())
}

#[test]
fn browser_area_application_service_descriptor_tracks_names_and_flags() -> Result<(), NetworkError>
{
    let descriptor = BrowseDescriptor::application_service("com.example.networkframework")?;
    let clone = descriptor.clone();
    assert_eq!(
        descriptor.application_service_name().as_deref(),
        Some("com.example.networkframework")
    );
    assert_eq!(
        clone.application_service_name().as_deref(),
        Some("com.example.networkframework")
    );

    let combined = BrowseResultChange::from_bits(
        BrowseResultChange::RESULT_ADDED.bits() | BrowseResultChange::TXT_RECORD_CHANGED.bits(),
    );
    assert!(combined.contains(BrowseResultChange::RESULT_ADDED));
    assert!(combined.contains(BrowseResultChange::TXT_RECORD_CHANGED));
    assert!(!combined.contains(BrowseResultChange::RESULT_REMOVED));
    Ok(())
}

#[test]
fn parameters_area_clone_keeps_protocol_stack_independent() -> Result<(), NetworkError> {
    let mut original = ConnectionParameters::tcp()?;
    let local_endpoint = Endpoint::address("127.0.0.1", 0)?;
    let websocket = ProtocolOptions::websocket()?;
    original
        .set_required_interface_type(InterfaceType::Loopback)
        .set_reuse_local_address(true)
        .set_local_endpoint(Some(&local_endpoint))
        .set_service_class(ServiceClass::ResponsiveData);
    original.prepend_application_protocol(&websocket)?;

    let cloned = original.clone();

    original
        .set_required_interface_type(InterfaceType::WiFi)
        .set_reuse_local_address(false)
        .set_local_endpoint(None)
        .set_service_class(ServiceClass::Background);

    let cloned_endpoint = cloned.local_endpoint().expect("cloned local endpoint");
    let cloned_stack = cloned
        .default_protocol_stack()
        .expect("cloned protocol stack");
    let cloned_protocols = cloned_stack.application_protocols();
    let websocket_definition = ProtocolDefinition::websocket()?;

    assert_eq!(cloned.required_interface_type(), InterfaceType::Loopback);
    assert!(cloned.reuse_local_address());
    assert_eq!(cloned.service_class(), ServiceClass::ResponsiveData);
    assert_eq!(cloned_endpoint.port(), 0);
    assert!(cloned_endpoint.address_string().is_some());
    assert_eq!(cloned_protocols.len(), 1);
    assert_eq!(cloned_protocols[0].definition(), Some(websocket_definition));

    assert_eq!(original.required_interface_type(), InterfaceType::WiFi);
    assert!(!original.reuse_local_address());
    assert!(original.local_endpoint().is_none());
    assert_eq!(original.service_class(), ServiceClass::Background);
    Ok(())
}

#[test]
fn endpoint_area_clone_preserves_signature_and_invalid_input_errors() -> Result<(), NetworkError> {
    let host = Endpoint::host("example.com", 8443)?;
    let host_clone = host.clone();
    assert_eq!(host_clone.endpoint_type(), EndpointType::Host);
    assert_eq!(host_clone.hostname(), host.hostname());
    assert_eq!(host_clone.port_string().as_deref(), Some("8443"));
    assert_eq!(host_clone.signature(), host.signature());

    let address = Endpoint::address("127.0.0.1", 8443)?;
    assert_eq!(address.endpoint_type(), EndpointType::Address);
    assert_eq!(address.port(), 8443);
    assert!(address.raw_address().is_some());

    assert!(matches!(
        Endpoint::host("bad\0host", 80),
        Err(NetworkError::InvalidArgument(_))
    ));
    assert!(matches!(
        Endpoint::url("https://exa\0mple.com"),
        Err(NetworkError::InvalidArgument(_))
    ));
    Ok(())
}

#[test]
fn path_area_clone_reports_loopback_interfaces() -> Result<(), NetworkError> {
    let listener = TcpListener::bind(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), NetworkError> {
        let connection = listener.accept()?;
        connection.send(b"path-ack")?;
        Ok(())
    });

    let mut parameters = ConnectionParameters::tcp()?;
    parameters.set_required_interface_type(InterfaceType::Loopback);
    let client = Connection::connect_with_parameters("127.0.0.1", port, &parameters)?;
    let path = client.current_path().expect("current path");
    let path_clone = path.clone();
    assert!(path_clone == path);
    assert!(path.uses_interface_type(InterfaceType::Loopback));
    assert!(path
        .interfaces()
        .iter()
        .any(|interface| interface.interface_type == InterfaceType::Loopback));
    assert_eq!(
        path.effective_remote_endpoint()
            .expect("remote endpoint")
            .port(),
        port
    );
    assert!(matches!(
        path.effective_local_endpoint()
            .expect("local endpoint")
            .endpoint_type(),
        EndpointType::Address | EndpointType::Host
    ));

    assert_eq!(client.receive(1024)?, b"path-ack");
    server.join().expect("server thread")?;
    Ok(())
}

#[test]
fn framer_area_message_metadata_round_trips_through_content_context() -> Result<(), NetworkError> {
    let definition =
        FramerDefinition::new(&unique_label("metadata-framer"), MetadataFramer::default)?;
    let options = definition.options()?;
    let mut message = options.create_message()?;
    message.set_u64("sequence", 42)?;
    assert_eq!(message.get_u64("sequence"), Some(42));

    let mut context = ContentContext::new("framer-context")?;
    context.set_framer_message(&message);
    let copied = context
        .copy_framer_message(&options)
        .expect("copied framer message");
    assert_eq!(copied.get_u64("sequence"), Some(42));
    assert_eq!(context.identifier(), "framer-context");
    Ok(())
}

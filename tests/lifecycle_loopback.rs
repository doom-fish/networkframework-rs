use std::io::{Read, Write};
use std::net::{Ipv6Addr, Shutdown, SocketAddr, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};

use networkframework::{
    certificate_sha256, ConnectionGroup, ConnectionGroupDescriptor, ConnectionGroupState,
    ConnectionParameters, Endpoint, Framer, FramerContext, FramerDefinition, FramerMessageView,
    FramerStart, NetworkError, ProtocolDefinition, ProtocolOptions, QuicConnection, TcpClient,
    TcpListener, TlsIdentity, TlsVersion, UdpClient,
};

fn loopback_only(mut parameters: ConnectionParameters) -> Result<ConnectionParameters, NetworkError> {
    parameters.set_local_endpoint(Some(&Endpoint::address("127.0.0.1", 0)?));
    Ok(parameters)
}

fn reset_connection(port: u16) {
    if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
        let socket = with_linger_zero(stream);
        drop(socket);
    }
}

fn with_linger_zero(stream: TcpStream) -> TcpStream {
    use std::os::fd::AsRawFd;
    let linger = Linger {
        l_onoff: 1,
        l_linger: 0,
    };
    unsafe {
        setsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_LINGER,
            std::ptr::addr_of!(linger).cast(),
            u32::try_from(std::mem::size_of::<Linger>()).expect("linger size"),
        );
    }
    stream
}

#[repr(C)]
struct Linger {
    l_onoff: i32,
    l_linger: i32,
}

const SOL_SOCKET: i32 = 0xffff;
const SO_LINGER: i32 = 0x0080;

unsafe extern "C" {
    fn setsockopt(
        socket: i32,
        level: i32,
        name: i32,
        value: *const core::ffi::c_void,
        len: u32,
    ) -> i32;
}

fn echo_once(connection: &TcpClient) -> Result<(), NetworkError> {
    let data = connection.receive(64)?;
    connection.send(&data)
}

#[test]
fn accept_survives_resets_and_serves_simultaneous_clients() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();

    for _ in 0..16 {
        reset_connection(port);
    }

    let mut clients = Vec::new();
    for index in 0..8 {
        clients.push(thread::spawn(move || -> Result<Vec<u8>, NetworkError> {
            let client = TcpClient::connect("127.0.0.1", port)?;
            let payload = format!("client-{index}");
            client.send(payload.as_bytes())?;
            client.receive(64)
        }));
    }

    let server = thread::spawn(move || -> Result<usize, NetworkError> {
        let mut served = 0;
        while served < 8 {
            let connection = listener.accept()?;
            if echo_once(&connection).is_ok() {
                served += 1;
            }
        }
        Ok(served)
    });

    let mut replies: Vec<String> = clients
        .into_iter()
        .map(|client| {
            let reply = client.join().expect("client thread")?;
            Ok(String::from_utf8(reply).expect("utf8 reply"))
        })
        .collect::<Result<_, NetworkError>>()?;
    replies.sort();
    let mut expected: Vec<String> = (0..8).map(|index| format!("client-{index}")).collect();
    expected.sort();
    assert_eq!(replies, expected);
    assert_eq!(server.join().expect("server thread")?, 8);
    Ok(())
}

#[test]
fn peer_reset_before_drop_is_torn_down_safely() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    for _ in 0..20 {
        let peer = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let connection = listener.accept()?;
        drop(with_linger_zero(peer));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut failed = false;
        while Instant::now() < deadline {
            match connection.receive(16) {
                Ok(data) if data.is_empty() => {
                    failed = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => {
                    failed = true;
                    break;
                }
            }
        }
        assert!(failed, "a reset peer must end the stream");
        drop(connection);
    }
    Ok(())
}

#[test]
fn repeated_create_and_drop_in_every_order() -> Result<(), NetworkError> {
    for round in 0..40 {
        let listener = TcpListener::bind_loopback(0)?;
        let port = listener.local_port();
        let mut client = TcpClient::connect("127.0.0.1", port)?;
        client.set_viability_changed_handler(|_| {});
        client.set_better_path_available_handler(|_| {});
        client.set_path_changed_handler(|_| {});
        client.send(b"x")?;
        match round % 4 {
            0 => {
                drop(client);
                drop(listener);
            }
            1 => {
                drop(listener);
                drop(client);
            }
            2 => {
                let accepted = listener.accept()?;
                drop(listener);
                drop(client);
                drop(accepted);
            }
            _ => {
                let accepted = listener.accept()?;
                client.force_cancel();
                drop(accepted);
                drop(client);
                drop(listener);
            }
        }
    }
    Ok(())
}

#[test]
fn replacing_handlers_while_events_arrive_is_safe() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    let mut client = TcpClient::connect("127.0.0.1", port)?;
    let calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..200 {
        let calls = Arc::clone(&calls);
        client.set_path_changed_handler(move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        });
    }
    drop(client);
    drop(listener);
    Ok(())
}

fn released_within(weak: &Weak<()>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while weak.strong_count() > 0 {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
fn dropped_clients_release_their_handler_contexts() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    let mut released = Vec::new();
    for _ in 0..8 {
        let mut client = TcpClient::connect("127.0.0.1", port)?;
        let token = Arc::new(());
        released.push(Arc::downgrade(&token));
        client.set_viability_changed_handler(move |_| {
            let _ = Arc::strong_count(&token);
        });
        let _accepted = listener.accept()?;
        drop(client);
    }
    for weak in &released {
        assert!(
            released_within(weak, Duration::from_secs(10)),
            "a dropped client kept its handler context alive"
        );
    }
    Ok(())
}

#[test]
fn oversized_udp_datagrams_are_reported_and_boundaries_kept() -> Result<(), NetworkError> {
    let server = UdpSocket::bind("127.0.0.1:0").expect("bind udp");
    server
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("read timeout");
    let port = server.local_addr().expect("local addr").port();
    let client = UdpClient::connect("127.0.0.1", port)?;
    client.send(b"hello")?;
    let mut buffer = [0_u8; 64];
    let (length, peer) = server.recv_from(&mut buffer).expect("recv");
    assert_eq!(&buffer[..length], b"hello");

    server.send_to(&[7_u8; 3000], peer).expect("send big");
    server.send_to(&[9_u8; 100], peer).expect("send small");

    match client.receive(1500) {
        Err(NetworkError::MessageTooLarge { size, limit }) => {
            assert_eq!(size, 3000);
            assert_eq!(limit, 1500);
        }
        other => panic!("expected MessageTooLarge, got {other:?}"),
    }
    let next = client.receive(1500)?;
    assert_eq!(next, vec![9_u8; 100]);
    Ok(())
}

struct CountingFramer;

impl Framer for CountingFramer {
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
fn framer_factory_outlives_rewrapped_parameters() -> Result<(), NetworkError> {
    let server = std::net::TcpListener::bind("127.0.0.1:0").expect("bind std listener");
    let port = server.local_addr().expect("addr").port();
    let accept_thread = thread::spawn(move || {
        let mut streams = Vec::new();
        for _ in 0..2 {
            if let Ok((stream, _)) = server.accept() {
                streams.push(stream);
            }
        }
        streams
    });

    let instances = Arc::new(AtomicUsize::new(0));
    let rewrapped = {
        let counter = Arc::clone(&instances);
        let definition = FramerDefinition::new("doomfish-counting", move || {
            counter.fetch_add(1, Ordering::SeqCst);
            CountingFramer
        })?;
        let options = definition.options()?;
        let mut parameters = ConnectionParameters::tcp()?;
        parameters.prepend_framer(&options)?;
        let client = TcpClient::connect_with_parameters("127.0.0.1", port, &parameters)?;
        let rewrapped = client.parameters().expect("connection parameters");
        drop(client);
        drop(parameters);
        drop(options);
        drop(definition);
        rewrapped
    };
    let before = instances.load(Ordering::SeqCst);
    assert!(before >= 1);
    let second = TcpClient::connect_with_parameters("127.0.0.1", port, &rewrapped)?;
    assert!(instances.load(Ordering::SeqCst) > before);
    drop(second);
    drop(rewrapped);
    let streams = accept_thread.join().expect("accept thread");
    assert_eq!(streams.len(), 2);
    Ok(())
}

struct AsyncReadyFramer {
    marked: Arc<AtomicUsize>,
}

impl Framer for AsyncReadyFramer {
    fn on_start(&mut self, context: &mut FramerContext) -> FramerStart {
        let marked = Arc::clone(&self.marked);
        context.async_invoke(move |context| {
            marked.fetch_add(1, Ordering::SeqCst);
            context.mark_ready();
        });
        FramerStart::WillMarkReady
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
fn framer_async_invocations_mark_the_connection_ready() -> Result<(), NetworkError> {
    let server = std::net::TcpListener::bind("127.0.0.1:0").expect("bind std listener");
    let port = server.local_addr().expect("addr").port();
    let accept_thread = thread::spawn(move || {
        let mut streams = Vec::new();
        for _ in 0..4 {
            if let Ok((stream, _)) = server.accept() {
                streams.push(stream);
            }
        }
        streams
    });

    let marked = Arc::new(AtomicUsize::new(0));
    let factory_marked = Arc::clone(&marked);
    let definition = FramerDefinition::new("doomfish-async-ready", move || AsyncReadyFramer {
        marked: Arc::clone(&factory_marked),
    })?;
    let options = definition.options()?;
    let mut parameters = ConnectionParameters::tcp()?;
    parameters.prepend_framer(&options)?;
    for round in 1..=4 {
        let client = TcpClient::connect_with_parameters("127.0.0.1", port, &parameters)?;
        assert!(marked.load(Ordering::SeqCst) >= round);
        drop(client);
    }
    let streams = accept_thread.join().expect("accept thread");
    assert_eq!(streams.len(), 4);
    Ok(())
}

fn openssl() -> Option<&'static Path> {
    let path = Path::new("/usr/bin/openssl");
    path.exists().then_some(path)
}

fn run(program: &Path, args: &[&str], dir: &Path) -> bool {
    Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|output| output.status.success())
}

struct TestIdentity {
    identity: TlsIdentity,
    pin: [u8; 32],
}

fn test_identity(label: &str) -> Option<TestIdentity> {
    let openssl = openssl()?;
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("tls-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let generated = run(
        openssl,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
            "-days",
            "2",
            "-subj",
            "/CN=localhost",
        ],
        &dir,
    ) && run(
        openssl,
        &[
            "pkcs12",
            "-export",
            "-inkey",
            "key.pem",
            "-in",
            "cert.pem",
            "-out",
            "identity.p12",
            "-passout",
            "pass:doomfish-test",
        ],
        &dir,
    ) && run(
        openssl,
        &[
            "x509", "-in", "cert.pem", "-outform", "der", "-out", "cert.der",
        ],
        &dir,
    );
    if !generated {
        eprintln!("skipping: openssl could not create a test identity");
        return None;
    }
    let der = std::fs::read(dir.join("cert.der")).ok()?;
    let pkcs12 = std::fs::read(dir.join("identity.p12")).ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    match TlsIdentity::from_pkcs12(&pkcs12, "doomfish-test") {
        Ok(identity) => Some(TestIdentity {
            identity,
            pin: certificate_sha256(&der),
        }),
        Err(NetworkError::Unsupported(reason)) => {
            eprintln!("skipping: {reason}");
            None
        }
        Err(error) => panic!("importing the test identity failed: {error}"),
    }
}

#[test]
fn pkcs12_import_rejects_garbage_and_wrong_passwords() {
    assert!(matches!(
        TlsIdentity::from_pkcs12(&[], "x"),
        Err(NetworkError::InvalidArgument(_))
    ));
    assert!(matches!(
        TlsIdentity::from_pkcs12(b"not a pkcs12 blob", "x\0y"),
        Err(NetworkError::InvalidArgument(_))
    ));
    match TlsIdentity::from_pkcs12(b"not a pkcs12 blob", "x") {
        Err(NetworkError::Security(_) | NetworkError::Unsupported(_)) => {}
        other => panic!("garbage PKCS#12 must fail cleanly, got {other:?}"),
    }
    let Some(openssl) = openssl() else {
        return;
    };
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("tls-password-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let generated = run(
        openssl,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
            "-days",
            "2",
            "-subj",
            "/CN=localhost",
        ],
        &dir,
    ) && run(
        openssl,
        &[
            "pkcs12",
            "-export",
            "-inkey",
            "key.pem",
            "-in",
            "cert.pem",
            "-out",
            "identity.p12",
            "-passout",
            "pass:right",
        ],
        &dir,
    );
    if generated {
        let pkcs12 = std::fs::read(dir.join("identity.p12")).expect("p12");
        match TlsIdentity::from_pkcs12(&pkcs12, "wrong") {
            Err(NetworkError::Security(_) | NetworkError::Unsupported(_)) => {}
            other => panic!("a wrong password must fail cleanly, got {other:?}"),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn pinned_client_parameters(
    pin: [u8; 32],
    alpn: Option<&str>,
) -> Result<ConnectionParameters, NetworkError> {
    let mut alpn_error = None;
    let parameters = ConnectionParameters::tls_tcp_configured(|tls| {
        tls.pin_peer_certificate_sha256(&[pin]);
        if let Some(alpn) = alpn {
            if let Err(error) = tls.add_application_protocol(alpn) {
                alpn_error = Some(error);
            }
        }
    })?;
    alpn_error.map_or(Ok(parameters), Err)
}

#[test]
fn tls_listener_completes_pinned_handshakes_and_survives_failed_ones() -> Result<(), NetworkError> {
    let Some(TestIdentity { identity, pin }) = test_identity("listener") else {
        return Ok(());
    };
    let server_parameters = ConnectionParameters::tls_tcp_configured(|tls| {
        tls.set_local_identity(&identity)
            .set_min_tls_version(TlsVersion::Tls12);
        tls.add_application_protocol("doomfish-echo")
            .expect("server ALPN");
    })?;
    let listener = TcpListener::bind_with_parameters(0, &loopback_only(server_parameters)?)?;
    let port = listener.local_port();

    let (tx, rx) = mpsc::channel();
    let server = thread::spawn(move || -> Result<(), NetworkError> {
        loop {
            let connection = listener.accept()?;
            let data = connection.receive(64)?;
            connection.send(&data)?;
            if data == b"done" {
                tx.send(()).expect("notify");
                return Ok(());
            }
        }
    });

    for _ in 0..10 {
        if let Ok(mut plain) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = plain.write_all(b"this is not a TLS ClientHello\r\n\r\n");
            let _ = plain.shutdown(Shutdown::Both);
        }
        reset_connection(port);
    }

    let mut wrong_pin = pin;
    wrong_pin[0] ^= 0xff;
    let rejected = TcpClient::connect_with_parameters(
        "127.0.0.1",
        port,
        &pinned_client_parameters(wrong_pin, None)?,
    );
    assert!(rejected.is_err(), "a pin mismatch must fail the handshake");

    let client = TcpClient::connect_with_parameters(
        "127.0.0.1",
        port,
        &pinned_client_parameters(pin, Some("doomfish-echo"))?,
    )?;
    client.send(b"hello over tls")?;
    assert_eq!(client.receive(64)?, b"hello over tls");

    let metadata = client
        .protocol_metadata(&ProtocolDefinition::tls()?)
        .expect("TLS metadata");
    let security = metadata.tls_security_metadata().expect("security metadata");
    assert!(matches!(
        security.negotiated_tls_version(),
        Some(TlsVersion::Tls12 | TlsVersion::Tls13)
    ));
    assert_eq!(
        security.negotiated_application_protocol().as_deref(),
        Some("doomfish-echo")
    );

    assert!(
        TcpClient::connect_tls("127.0.0.1", port).is_err(),
        "a self-signed server must fail default trust"
    );

    let finisher = TcpClient::connect_with_parameters(
        "127.0.0.1",
        port,
        &pinned_client_parameters(pin, Some("doomfish-echo"))?,
    )?;
    finisher.send(b"done")?;
    assert_eq!(finisher.receive(64)?, b"done");
    rx.recv_timeout(Duration::from_secs(10))
        .expect("server finished");
    server.join().expect("server thread")?;
    Ok(())
}

#[test]
#[ignore = "bind_tls listens on every interface"]
fn bind_tls_serves_a_pinned_client() -> Result<(), NetworkError> {
    let Some(TestIdentity { identity, pin }) = test_identity("bind") else {
        return Ok(());
    };
    let listener = Arc::new(TcpListener::bind_tls(0, &identity)?);
    let port = listener.local_port();
    let server_listener = Arc::clone(&listener);
    let server = thread::spawn(move || -> Result<(), NetworkError> {
        let connection = server_listener.accept()?;
        echo_once(&connection)
    });
    let client = TcpClient::connect_with_parameters(
        "127.0.0.1",
        port,
        &pinned_client_parameters(pin, None)?,
    )?;
    client.send(b"ping")?;
    assert_eq!(client.receive(64)?, b"ping");
    server.join().expect("server thread")?;

    let started = Instant::now();
    let default_trust = TcpClient::connect_tls("127.0.0.1", port);
    assert!(
        default_trust.is_err(),
        "a self-signed server must fail default trust"
    );
    eprintln!("default trust rejected after {:?}", started.elapsed());
    drop(listener);
    Ok(())
}

#[test]
fn quic_uses_real_quic_parameters() -> Result<(), NetworkError> {
    let mut parameters = ConnectionParameters::quic("doomfish-quic")?;
    let stack = parameters.default_protocol_stack().expect("protocol stack");
    assert!(stack.application_protocols().is_empty());
    assert!(stack
        .transport_protocol()
        .is_some_and(|transport| transport.is_quic()));
    drop(stack);

    let Some(TestIdentity { identity, pin }) = test_identity("quic") else {
        return Ok(());
    };
    let server_parameters = ConnectionParameters::quic_configured("doomfish-quic", |tls| {
        tls.set_local_identity(&identity);
    })?;
    let listener = Arc::new(TcpListener::bind_with_parameters(
        0,
        &loopback_only(server_parameters)?,
    )?);
    let port = listener.local_port();
    let (echoed_tx, echoed_rx) = mpsc::channel();
    let server_listener = Arc::clone(&listener);
    thread::spawn(move || {
        while let Ok(connection) = server_listener.accept() {
            let echoed_tx = echoed_tx.clone();
            thread::spawn(move || {
                if echo_once(&connection).is_ok() {
                    let _ = echoed_tx.send(());
                }
            });
        }
    });
    let client_parameters = ConnectionParameters::quic_configured("doomfish-quic", |tls| {
        tls.pin_peer_certificate_sha256(&[pin]);
    })?;
    let client = QuicConnection::connect_with_parameters("127.0.0.1", port, &client_parameters)?;
    client.send(b"quic ping")?;
    assert_eq!(client.receive(64)?, b"quic ping");
    echoed_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the server echoed on the QUIC stream");
    drop(client);
    drop(listener);
    Ok(())
}

#[test]
fn quic_multiplex_group_starts_rejects_late_handlers_and_cancels() -> Result<(), NetworkError> {
    let Some(TestIdentity { identity, pin }) = test_identity("group") else {
        return Ok(());
    };
    let server_parameters = ConnectionParameters::quic_configured("doomfish-group", |tls| {
        tls.set_local_identity(&identity);
    })?;
    let listener = TcpListener::bind_with_parameters(0, &loopback_only(server_parameters)?)?;
    let client_parameters = ConnectionParameters::quic_configured("doomfish-group", |tls| {
        tls.pin_peer_certificate_sha256(&[pin]);
    })?;
    for round in 0..3 {
        let descriptor = ConnectionGroupDescriptor::multiplex("127.0.0.1", listener.local_port())?;
        let mut group = ConnectionGroup::new(descriptor, &client_parameters)?;
        let (state_tx, state_rx) = mpsc::channel();
        group.set_state_changed_handler(move |state| {
            let _ = state_tx.send(state);
        });
        group.set_new_connection_handler(|_connection| {})?;
        group.start()?;
        assert!(matches!(
            group.set_new_connection_handler(|_connection| {}),
            Err(NetworkError::InvalidArgument(_))
        ));
        assert!(matches!(group.start(), Err(NetworkError::InvalidArgument(_))));
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut ready = false;
        while !ready {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match state_rx.recv_timeout(remaining) {
                Ok(ConnectionGroupState::Ready) => ready = true,
                Ok(ConnectionGroupState::Failed | ConnectionGroupState::Cancelled) | Err(_) => {
                    panic!("round {round}: the multiplex group did not become ready");
                }
                Ok(_) => {}
            }
        }
        group.cancel();
        let cancelled = loop {
            match state_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(ConnectionGroupState::Cancelled) => break true,
                Ok(_) => {}
                Err(_) => break false,
            }
        };
        assert!(cancelled, "round {round}: cancel must deliver the final cancelled state");
        drop(group);
    }
    drop(listener);
    Ok(())
}

fn wait_for_group_state(
    states: &mpsc::Receiver<ConnectionGroupState>,
    wanted: ConnectionGroupState,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match states.recv_timeout(remaining) {
            Ok(state) if state == wanted => return true,
            Ok(ConnectionGroupState::Failed | ConnectionGroupState::Cancelled) | Err(_) => {
                return false;
            }
            Ok(_) => {}
        }
    }
}

#[test]
fn reinsertion_releases_the_extracted_connection_while_the_group_lives() -> Result<(), NetworkError>
{
    let Some(TestIdentity { identity, pin }) = test_identity("reinsert") else {
        return Ok(());
    };
    let server_parameters = ConnectionParameters::quic_configured("doomfish-reinsert", |tls| {
        tls.set_local_identity(&identity);
    })?;
    let listener = TcpListener::bind_with_parameters(0, &loopback_only(server_parameters)?)?;
    let port = listener.local_port();
    let client_parameters = ConnectionParameters::quic_configured("doomfish-reinsert", |tls| {
        tls.pin_peer_certificate_sha256(&[pin]);
    })?;
    let descriptor = ConnectionGroupDescriptor::multiplex("127.0.0.1", port)?;
    let mut group = ConnectionGroup::new(descriptor, &client_parameters)?;
    let (state_tx, state_rx) = mpsc::channel();
    group.set_state_changed_handler(move |state| {
        let _ = state_tx.send(state);
    });
    group.set_new_connection_handler(|_connection| {})?;
    group.start()?;
    assert!(
        wait_for_group_state(
            &state_rx,
            ConnectionGroupState::Ready,
            Duration::from_secs(10)
        ),
        "the multiplex group did not become ready"
    );

    let stream_options = ProtocolOptions::quic()?;
    let mut released = Vec::new();
    for round in 0..3 {
        let mut extracted = match group.extract_connection(None, Some(&stream_options)) {
            Ok(extracted) => extracted,
            Err(NetworkError::ConnectFailed) if round == 0 => {
                eprintln!(
                    "skip: this system cannot extract a stream from a QUIC multiplex group (connect failed)"
                );
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let token = Arc::new(());
        released.push(Arc::downgrade(&token));
        extracted.set_viability_changed_handler(move |_| {
            let _ = Arc::strong_count(&token);
        });
        match group.reinsert_extracted_connection(extracted) {
            Ok(()) | Err(NetworkError::InvalidArgument(_)) => {}
            Err(error) => return Err(error),
        }
    }
    for weak in &released {
        assert!(
            released_within(weak, Duration::from_secs(10)),
            "a reinsertion kept the extracted connection's shim handle while the group was alive"
        );
    }

    group.cancel();
    assert!(wait_for_group_state(
        &state_rx,
        ConnectionGroupState::Cancelled,
        Duration::from_secs(5)
    ));
    drop(group);
    drop(listener);
    Ok(())
}

#[test]
fn group_listener_delivers_groups_and_refuses_accept() -> Result<(), NetworkError> {
    let Some(TestIdentity { identity, pin }) = test_identity("group-listener") else {
        return Ok(());
    };
    let server_parameters = ConnectionParameters::quic_configured("doomfish-groups", |tls| {
        tls.set_local_identity(&identity);
    })?;
    let (group_tx, group_rx) = mpsc::channel();
    let listener = TcpListener::bind_with_group_handler(
        0,
        &loopback_only(server_parameters)?,
        move |group| {
            let _ = group_tx.send(group);
        },
    )?;
    assert!(matches!(
        listener.accept(),
        Err(NetworkError::InvalidArgument(_))
    ));

    let client_parameters = ConnectionParameters::quic_configured("doomfish-groups", |tls| {
        tls.pin_peer_certificate_sha256(&[pin]);
    })?;
    let descriptor = ConnectionGroupDescriptor::multiplex("127.0.0.1", listener.local_port())?;
    let mut group = ConnectionGroup::new(descriptor, &client_parameters)?;
    group.set_new_connection_handler(|_connection| {})?;
    let group = Arc::new(group);
    let starter = {
        let group = Arc::clone(&group);
        thread::spawn(move || group.start())
    };
    let delivered = group_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the listener delivers the inbound QUIC connection as a group");
    assert!(delivered.descriptor().is_some());
    group.cancel();
    let _ = starter.join().expect("starter thread");
    drop(delivered);
    drop(group);
    drop(listener);
    Ok(())
}

#[test]
fn loopback_listener_binds_only_the_loopback_address() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    let mut local = TcpStream::connect(("127.0.0.1", port)).expect("loopback connect");
    local.write_all(b"x").expect("write");
    let ipv6_loopback = SocketAddr::from((Ipv6Addr::LOCALHOST, port));
    assert!(TcpStream::connect_timeout(&ipv6_loopback, Duration::from_secs(2)).is_err());
    let accepted = listener.accept()?;
    assert_eq!(accepted.receive(1)?, b"x");
    let mut sink = [0_u8; 1];
    drop(accepted);
    let _ = local.read(&mut sink);
    Ok(())
}

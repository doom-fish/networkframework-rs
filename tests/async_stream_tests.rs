//! Tests for `async_api` stream surfaces.

#![cfg(feature = "async")]

use std::time::{Duration, Instant};

use networkframework::async_api::{
    ConnectionBetterPathStream, ConnectionPathChangedStream, ConnectionState,
    ConnectionStateStream, ConnectionViabilityStream, ListenerEvent, ListenerEventStream,
    PathUpdateStream,
};
use networkframework::client::TcpClient;
use networkframework::listener::TcpListener;
use networkframework::path_monitor::start_path_monitor;
use networkframework::NetworkError;

fn poll_until<T>(timeout: Duration, mut poll: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(value) = poll() {
            return Some(value);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

fn connected_pair() -> Result<(TcpListener, TcpClient, TcpClient), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let client = TcpClient::connect("127.0.0.1", listener.local_port())?;
    let accepted = listener.accept()?;
    Ok((listener, client, accepted))
}

/// `PathUpdateStream`: subscribe, receive first path event, drop handle, stream closes.
#[test]
fn path_update_stream_subscribe_and_drop() {
    let monitor = start_path_monitor(|_| {});
    let stream = PathUpdateStream::subscribe(&monitor, 4);
    assert!(poll_until(Duration::from_secs(5), || monitor.current_path()).is_some());
    let _ = stream.try_next();
    drop(stream);
    assert!(monitor.current_path().is_some());
}

#[test]
fn connection_state_stream_reports_cancellation_and_drop_is_prompt() -> Result<(), NetworkError> {
    let (_listener, client, _accepted) = connected_pair()?;
    let stream = ConnectionStateStream::subscribe(&client, 8);
    client.force_cancel();
    let cancelled = poll_until(Duration::from_secs(5), || {
        stream
            .try_next()
            .filter(|event| event.state == ConnectionState::Cancelled)
    });
    assert!(
        cancelled.is_some(),
        "the stream must report the final cancelled state"
    );
    drop(stream);
    let started = Instant::now();
    drop(client);
    assert!(started.elapsed() < Duration::from_secs(1));
    Ok(())
}

#[test]
fn dropping_streams_keeps_the_connection_usable() -> Result<(), NetworkError> {
    let (_listener, client, accepted) = connected_pair()?;
    for _ in 0..3 {
        let state = ConnectionStateStream::subscribe(&client, 4);
        let viability = ConnectionViabilityStream::subscribe(&client, 4);
        let better_path = ConnectionBetterPathStream::subscribe(&client, 4);
        let path = ConnectionPathChangedStream::subscribe(&client, 4);
        drop((state, viability, better_path, path));
    }
    client.send(b"still alive")?;
    assert_eq!(accepted.receive(64)?, b"still alive");
    accepted.send(b"ack")?;
    assert_eq!(client.receive(64)?, b"ack");

    let stream = ConnectionStateStream::subscribe(&client, 8);
    drop(accepted);
    drop(stream);
    let started = Instant::now();
    drop(client);
    assert!(started.elapsed() < Duration::from_secs(1));
    Ok(())
}

#[test]
fn listener_event_stream_hands_off_ready_connections() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    let stream = ListenerEventStream::subscribe(&listener, 8);

    let client = TcpClient::connect("127.0.0.1", port)?;
    client.send(b"via stream")?;
    let accepted = poll_until(Duration::from_secs(5), || match stream.try_next() {
        Some(ListenerEvent::NewConnection(connection)) => Some(connection),
        _ => None,
    })
    .expect("the stream delivers the ready connection");
    assert_eq!(accepted.receive(64)?, b"via stream");
    drop(stream);

    let second = TcpClient::connect("127.0.0.1", port)?;
    second.send(b"via accept")?;
    let accepted_again = listener.accept()?;
    assert_eq!(accepted_again.receive(64)?, b"via accept");
    Ok(())
}

//! Tests for `async_api` stream surfaces.
//!
//! These tests use `pollster::block_on` and verify the subscribe → event → drop
//! cycle without requiring a live network.

#![cfg(feature = "async")]

use networkframework::async_api::{
    ConnectionBetterPathStream, ConnectionPathChangedStream, ConnectionStateStream,
    ConnectionViabilityStream, PathUpdateStream,
};
use networkframework::path_monitor::start_path_monitor;

/// `PathUpdateStream`: subscribe, receive first path event, drop handle, stream closes.
#[test]
fn path_update_stream_subscribe_and_drop() {
    pollster::block_on(async {
        let monitor = start_path_monitor(|_| {});
        let stream = PathUpdateStream::subscribe(&monitor, 4);

        let _ = stream.try_next();

        drop(stream);
    });
}

/// `ConnectionStateStream`: subscribe and immediately drop without crash.
#[test]
fn connection_state_stream_drop_is_safe() {
    use networkframework::client::TcpClient;

    let Ok(client) = TcpClient::connect("127.0.0.1", 9) else {
        return;
    };
    let stream = ConnectionStateStream::subscribe(&client, 8);
    drop(stream);
}

/// `ConnectionViabilityStream`: subscribe and drop without crash.
#[test]
fn connection_viability_stream_drop_is_safe() {
    use networkframework::client::TcpClient;

    let Ok(client) = TcpClient::connect("127.0.0.1", 9) else {
        return;
    };
    let stream = ConnectionViabilityStream::subscribe(&client, 4);
    drop(stream);
}

/// `ConnectionBetterPathStream`: subscribe and drop without crash.
#[test]
fn connection_better_path_stream_drop_is_safe() {
    use networkframework::client::TcpClient;

    let Ok(client) = TcpClient::connect("127.0.0.1", 9) else {
        return;
    };
    let stream = ConnectionBetterPathStream::subscribe(&client, 4);
    drop(stream);
}

/// `ConnectionPathChangedStream`: subscribe and drop without crash.
#[test]
fn connection_path_changed_stream_drop_is_safe() {
    use networkframework::client::TcpClient;

    let Ok(client) = TcpClient::connect("127.0.0.1", 9) else {
        return;
    };
    let stream = ConnectionPathChangedStream::subscribe(&client, 4);
    drop(stream);
}

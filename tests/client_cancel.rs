use std::io::Read;
use std::net::TcpListener as StdListener;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use networkframework::{NetworkError, TcpClient};

/// A std loopback listener that accepts one connection and reports what
/// `read_to_end` returned once the peer closed.
fn watch_one_close() -> (u16, mpsc::Receiver<std::io::Result<usize>>) {
    let server = StdListener::bind("127.0.0.1:0").expect("bind");
    let port = server.local_addr().expect("local addr").port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = server.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("read timeout");
        let mut rest = Vec::new();
        let _ = tx.send(stream.read_to_end(&mut rest));
    });
    (port, rx)
}

#[test]
fn cancel_closes_gracefully_so_the_peer_sees_eof() -> Result<(), NetworkError> {
    let (port, closed) = watch_one_close();
    let client = TcpClient::connect("127.0.0.1", port)?;
    client.cancel();
    let result = closed
        .recv_timeout(Duration::from_secs(10))
        .expect("peer never saw the close");
    assert!(
        matches!(result, Ok(0)),
        "expected a clean EOF, got {result:?}"
    );
    Ok(())
}

#[test]
fn cancel_wakes_a_receive_blocked_on_another_thread() -> Result<(), NetworkError> {
    let (port, _closed) = watch_one_close();
    let client = Arc::new(TcpClient::connect("127.0.0.1", port)?);
    let reader = Arc::clone(&client);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(reader.receive(1024));
    });
    thread::sleep(Duration::from_millis(200));
    client.cancel();
    let received = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("receive stayed blocked after cancel");
    assert!(
        !matches!(&received, Ok(data) if !data.is_empty()),
        "unexpected data {received:?}"
    );
    Ok(())
}

#[test]
fn cancel_twice_and_then_drop_is_harmless() -> Result<(), NetworkError> {
    let (port, _closed) = watch_one_close();
    let client = TcpClient::connect("127.0.0.1", port)?;
    client.cancel();
    client.cancel();
    drop(client);
    Ok(())
}

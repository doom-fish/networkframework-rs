//! Demonstrates async event streams for Network.framework objects.
//!
//! Runs a short-lived path-monitor stream and a connection state stream,
//! exits cleanly on a headless macOS machine.

#[cfg(feature = "async")]
fn main() {
    use networkframework::async_api::{ConnectionStateStream, PathUpdateStream};
    use networkframework::client::TcpClient;
    use networkframework::path_monitor::start_path_monitor;

    pollster::block_on(async {
        let monitor = start_path_monitor(|_| {});
        let path_stream = PathUpdateStream::subscribe(&monitor, 8);
        if let Some(path) = path_stream.try_next() {
            println!(
                "Initial path satisfied={}",
                path.status() == networkframework::PathStatus::Satisfied,
            );
        } else {
            println!("No immediate path update (headless ok)");
        }

        match TcpClient::connect("93.184.216.34", 80) {
            Ok(client) => {
                let state_stream = ConnectionStateStream::subscribe(&client, 8);
                while let Some(event) = state_stream.try_next() {
                    println!("Connection state: {:?}", event.state);
                }
            }
            Err(error) => {
                println!("Connect failed (headless ok): {error}");
            }
        }

        drop(path_stream);
        drop(monitor);
        println!("async_streams example complete");
    });
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!("Run with --features async");
}

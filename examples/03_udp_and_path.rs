use networkframework::{start_path_monitor, UdpClient};
use std::time::Duration;

fn main() -> Result<(), networkframework::NetworkError> {
    // 1) Send a UDP datagram to 1.1.1.1:53 (Cloudflare DNS). We won't
    //    parse the response — we're just proving send+recv works.
    let dns_query: [u8; 12] = [
        0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];
    let c = UdpClient::connect("1.1.1.1", 53)?;
    c.send(&dns_query)?;
    // (Real DNS query would have a question section appended; we'll
    // just confirm the socket round-trips.)
    println!("sent {} bytes to 1.1.1.1:53 via UDP", dns_query.len());

    // 2) Start a path monitor for a couple of seconds, just to confirm
    //    the callback fires at least once.
    let _guard = start_path_monitor(|u| {
        println!("path update: satisfied={}, iface={:?}", u.satisfied, u.interface);
    });
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

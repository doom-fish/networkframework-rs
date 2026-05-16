use networkframework::TcpClient;

fn main() -> Result<(), networkframework::NetworkError> {
    let c = TcpClient::connect_tls("example.com", 443)?;
    c.send(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")?;
    let r = c.receive(2048)?;
    let s = String::from_utf8_lossy(&r);
    let line = s.lines().next().unwrap_or("");
    println!("{line} ({} bytes over TLS)", r.len());
    assert!(line.starts_with("HTTP/1."));
    Ok(())
}

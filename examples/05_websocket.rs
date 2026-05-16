use networkframework::{Opcode, WebSocket};

fn main() -> Result<(), networkframework::NetworkError> {
    // Postman's public echo server at wss://ws.postman-echo.com/raw
    let ws = WebSocket::connect("ws.postman-echo.com", 443, "/raw", true)?;
    ws.send_text("hello from networkframework-rs")?;
    let msg = ws.receive(4096)?;
    let text = String::from_utf8_lossy(&msg.data);
    println!("got {:?} message: {text} ({} bytes)", msg.opcode, msg.data.len());
    assert_eq!(msg.opcode, Opcode::Text);
    Ok(())
}

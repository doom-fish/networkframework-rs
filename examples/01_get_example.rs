use networkframework::{TcpClient, TcpListener};

fn main() -> Result<(), networkframework::NetworkError> {
    let listener = TcpListener::bind(0)?;
    let port = listener.local_port();
    let server = std::thread::spawn(move || -> Result<(), networkframework::NetworkError> {
        let connection = listener.accept()?;
        let request = connection.receive(1024)?;
        println!("server received: {}", String::from_utf8_lossy(&request));
        connection.send(b"pong")?;
        Ok(())
    });

    let client = TcpClient::connect("127.0.0.1", port)?;
    let endpoint = client.endpoint().expect("endpoint");
    println!("connected to {:?}:{}", endpoint.hostname(), endpoint.port());
    client.send(b"ping")?;
    let reply = client.receive(1024)?;
    println!("client received: {}", String::from_utf8_lossy(&reply));

    server.join().expect("server thread")?;
    Ok(())
}

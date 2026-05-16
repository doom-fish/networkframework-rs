use networkframework::{ProtocolDefinition, ProtocolOptions};

fn main() -> Result<(), networkframework::NetworkError> {
    let websocket_definition = ProtocolDefinition::websocket()?;
    let websocket_options = ProtocolOptions::websocket()?;
    println!(
        "websocket definition matches options: {}",
        websocket_options
            .definition()
            .as_ref()
            .is_some_and(|definition| definition == &websocket_definition)
    );

    let quic_options = ProtocolOptions::quic()?;
    println!("quic options report is_quic={}", quic_options.is_quic());
    Ok(())
}

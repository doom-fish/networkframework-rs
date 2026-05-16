use networkframework::QuicOptions;

fn main() -> Result<(), networkframework::NetworkError> {
    let mut options = QuicOptions::new()?;
    options
        .add_tls_application_protocol("h3")?
        .set_stream_is_unidirectional(true)
        .set_stream_is_datagram(true)
        .set_initial_max_data(65_536)
        .set_max_udp_payload_size(1350)
        .set_idle_timeout(10_000);

    println!(
        "quic: unidirectional={} datagram={} initial_max_data={} max_udp_payload={} idle_timeout={}",
        options.stream_is_unidirectional(),
        options.stream_is_datagram(),
        options.initial_max_data(),
        options.max_udp_payload_size(),
        options.idle_timeout(),
    );
    Ok(())
}

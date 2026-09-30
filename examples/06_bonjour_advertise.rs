use networkframework::{AdvertiseDescriptor, ConnectionParameters, TcpListener, TxtRecord};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<(), networkframework::NetworkError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_millis();
    let service_name = format!("networkframework-demo-{stamp}");
    let mut txt_record = TxtRecord::dictionary()?;
    txt_record.set_key("example", Some(b"1"))?;
    let mut descriptor =
        AdvertiseDescriptor::bonjour_service(Some(&service_name), "_nfwtest._tcp", Some("local"))?;
    descriptor
        .set_txt_record_object(&txt_record)
        .set_no_auto_rename(true);
    println!(
        "advertising service={:?} type={:?} domain={:?} no_auto_rename={}",
        descriptor.service_name(),
        descriptor.service_type(),
        descriptor.domain(),
        descriptor.no_auto_rename(),
    );

    // Enable peer-to-peer to advertise over AWDL as well.
    let mut parameters = ConnectionParameters::tcp()?;
    parameters.set_include_peer_to_peer(true);
    let listener = TcpListener::builder(&parameters)
        .advertise(descriptor)
        .on_advertised_endpoint(|endpoint, added| {
            let name = endpoint.and_then(|endpoint| endpoint.bonjour_service_name());
            println!("advertised={added} name={name:?}");
        })
        .bind()?;
    println!("listening on port {}", listener.local_port());
    std::thread::sleep(Duration::from_secs(2));
    Ok(())
}

use networkframework::{advertise_with_descriptor, AdvertiseDescriptor};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<(), networkframework::NetworkError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_millis();
    let service_name = format!("networkframework-demo-{stamp}");
    let mut descriptor =
        AdvertiseDescriptor::bonjour_service(Some(&service_name), "_nfwtest._tcp", Some("local"))?;
    descriptor
        .set_txt_record(b"example=1")
        .set_no_auto_rename(true);
    println!(
        "advertising service={:?} type={:?} domain={:?} no_auto_rename={}",
        descriptor.service_name(),
        descriptor.service_type(),
        descriptor.domain(),
        descriptor.no_auto_rename(),
    );
    let _advertiser = advertise_with_descriptor(&descriptor, 18_080)?;
    std::thread::sleep(Duration::from_millis(250));
    Ok(())
}

use networkframework::{start_browser_with_descriptor, BrowseDescriptor};
use std::time::Duration;

fn main() -> Result<(), networkframework::NetworkError> {
    let mut descriptor = BrowseDescriptor::bonjour_service("_http._tcp", Some("local"))?;
    descriptor.set_include_txt_record(true);
    println!(
        "browsing type={:?} domain={:?} include_txt_record={}",
        descriptor.bonjour_service_type(),
        descriptor.bonjour_service_domain(),
        descriptor.include_txt_record(),
    );
    let _browser = start_browser_with_descriptor(&descriptor, None, |event| {
        println!("browser event: {event:?}");
    })?;
    std::thread::sleep(Duration::from_millis(250));
    Ok(())
}

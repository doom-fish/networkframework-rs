use networkframework::{start_browser, BrowserEvent};
use std::time::Duration;

fn main() -> Result<(), networkframework::NetworkError> {
    // Look for a few common Bonjour service types.
    for service in ["_airplay._tcp", "_http._tcp", "_ssh._tcp", "_raop._tcp"] {
        println!("\nbrowsing {service} for 2s…");
        let svc_copy = service.to_string();
        let _guard = start_browser(service, None, move |evt| match evt {
            BrowserEvent::Found(s) => {
                println!("  + [{}] {:?} in {}", svc_copy, s.name, s.domain);
            }
            BrowserEvent::Lost(s) => {
                println!("  - [{}] {:?}", svc_copy, s.name);
            }
        })?;
        std::thread::sleep(Duration::from_secs(2));
    }
    Ok(())
}

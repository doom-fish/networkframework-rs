use networkframework::{advertise_bonjour_service, start_browser, BrowserEvent};
use std::time::Duration;

fn main() -> Result<(), networkframework::NetworkError> {
    let _adv = advertise_bonjour_service("_http._tcp", "doom-fish-test-service", None, 18080)?;
    println!("advertising _http._tcp as 'doom-fish-test-service' on port 18080");
    // Now browse for it and confirm we see ourselves.
    let _browser = start_browser("_http._tcp", None, |evt| {
        if let BrowserEvent::Found(s) = evt {
            if s.name.contains("doom-fish-test-service") {
                println!("  ✓ discovered our own advert: {:?}", s.name);
            }
        }
    })?;
    std::thread::sleep(Duration::from_secs(3));
    Ok(())
}

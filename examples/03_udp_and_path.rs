use networkframework::{start_path_monitor, ConnectionParameters, Endpoint};
use std::time::Duration;

fn main() -> Result<(), networkframework::NetworkError> {
    let _udp = ConnectionParameters::udp()?;
    let host = Endpoint::host("example.com", 443)?;
    let bonjour = Endpoint::bonjour_service(Some("demo"), "_http._tcp", Some("local"))?;
    let url = Endpoint::url("https://example.com")?;
    println!(
        "host endpoint: {:?}:{:?}",
        host.hostname(),
        host.port_string()
    );
    println!("bonjour endpoint: {:?}", bonjour.bonjour_service_name());
    println!("url endpoint: {:?}", url.url_string());

    let monitor = start_path_monitor(|update| {
        println!(
            "path update: satisfied={} interface={:?}",
            update.satisfied, update.interface
        );
    });
    std::thread::sleep(Duration::from_millis(250));
    if let Some(path) = monitor.current_path() {
        println!(
            "current path: status={:?} interfaces={} ipv4={} ipv6={} dns={}",
            path.status(),
            path.interfaces().len(),
            path.has_ipv4(),
            path.has_ipv6(),
            path.has_dns(),
        );
    }
    Ok(())
}

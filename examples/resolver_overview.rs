use networkframework::ResolverConfig;

fn main() -> Result<(), networkframework::NetworkError> {
    let mut doh = ResolverConfig::dns_over_https("https://example.com/dns-query{?dns}")?;
    doh.add_server_address("1.1.1.1", 443)?;

    let mut dot = ResolverConfig::dns_over_tls("dns.example", 853)?;
    dot.add_server_address("9.9.9.9", 853)?;

    println!("constructed DNS-over-HTTPS and DNS-over-TLS resolver configurations");
    Ok(())
}

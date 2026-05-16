use networkframework::{PrivacyContext, ProxyConfig, ResolverConfig};

fn main() -> Result<(), networkframework::NetworkError> {
    let resolver = ResolverConfig::dns_over_https("https://example.com/dns-query{?dns}")?;
    let privacy = PrivacyContext::new("networkframework-demo")?;
    privacy.require_encrypted_name_resolution(true, Some(&resolver));

    let mut proxy = ProxyConfig::http_connect("proxy.example", 443, true)?;
    proxy
        .set_credentials("demo", Some("secret"))?
        .set_failover_allowed(true)
        .add_match_domain("example.com")?
        .add_excluded_domain("internal.example.com")?;
    privacy.add_proxy(&proxy);

    println!(
        "privacy context configured with match_domains={:?} excluded_domains={:?}",
        proxy.match_domains(),
        proxy.excluded_domains(),
    );
    privacy.clear_proxies();
    Ok(())
}

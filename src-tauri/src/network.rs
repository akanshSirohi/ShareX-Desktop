/// Local IPv4 interfaces, preferring the interface selected by the default route.
pub fn addresses() -> Vec<String> {
    let mut addresses = Vec::new();
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        for (_, address) in interfaces {
            if address.is_ipv4() && !address.is_loopback() && !address.is_unspecified() {
                addresses.push(address.to_string());
            }
        }
    }
    let primary = local_ip_address::local_ip().ok().map(|ip| ip.to_string());
    addresses.sort_by_key(|ip| {
        (
            primary.as_ref() != Some(ip),
            ip.starts_with("169.254."),
            ip.clone(),
        )
    });
    addresses.dedup();
    addresses
}

//! Client-IP extraction with trusted-proxy support.
//!
//! `axum::extract::ConnectInfo` always returns the direct TCP peer.
//! Behind nginx / caddy / k8s ingress that's the proxy, not the
//! actual user. This helper checks whether the peer falls inside
//! one of the configured `trusted_proxies` CIDRs; when it does,
//! returns the last entry in `X-Forwarded-For`. Otherwise it
//! returns the peer.
//!
//! `trusted_proxies` is parsed once at boot from the
//! `OPEN4X_LOBBY_TRUSTED_PROXIES` env var (comma-separated CIDR
//! list, e.g. `127.0.0.0/8,10.0.0.0/8`). Empty list = trust no
//! proxy = always use the peer (default for dev).

#![cfg(feature = "ssr")]

use std::net::IpAddr;

use axum::http::HeaderMap;
use ipnet::IpNet;

/// Parse a comma-separated CIDR list. Whitespace tolerant; ignores
/// empty segments. Bad entries are skipped with a stderr warning so
/// the binary boots even if one entry is malformed.
pub fn parse_trusted_proxies(raw: &str) -> Vec<IpNet> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| match s.parse::<IpNet>() {
            Ok(n) => Some(n),
            Err(e) => {
                eprintln!("[trusted_proxies] skipping invalid CIDR {s:?}: {e}");
                None
            }
        })
        .collect()
}

/// Pick the right IP for rate-limit / audit-log keying.
pub fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> String {
    if trusted.iter().any(|net| net.contains(&peer)) {
        if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            // X-Forwarded-For: client, proxy1, proxy2, … — the
            // RIGHTMOST entry is the most-recent proxy. Walk
            // right-to-left and pick the first untrusted hop.
            for entry in forwarded
                .rsplit(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let Ok(ip) = entry.parse::<IpAddr>() else {
                    continue;
                };
                if !trusted.iter().any(|net| net.contains(&ip)) {
                    return ip.to_string();
                }
            }
        }
    }
    peer.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn h(k: &'static str, v: &str) -> HeaderMap {
        let mut m = HeaderMap::new();
        m.insert(k, HeaderValue::from_str(v).unwrap());
        m
    }

    #[test]
    fn untrusted_peer_returns_peer() {
        let trusted: Vec<IpNet> = vec!["127.0.0.0/8".parse().unwrap()];
        let peer: IpAddr = "203.0.113.5".parse().unwrap();
        let headers = h("x-forwarded-for", "1.2.3.4, 5.6.7.8");
        // Peer is not in any trusted CIDR -> ignore the header.
        assert_eq!(client_ip(peer, &headers, &trusted), "203.0.113.5");
    }

    #[test]
    fn trusted_peer_uses_xff_rightmost_untrusted() {
        let trusted: Vec<IpNet> = vec![
            "127.0.0.0/8".parse().unwrap(),
            "10.0.0.0/8".parse().unwrap(),
        ];
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        // Hops: client 198.51.100.7, proxy 10.0.0.1, proxy 127.0.0.1.
        let headers = h("x-forwarded-for", "198.51.100.7, 10.0.0.1");
        assert_eq!(client_ip(peer, &headers, &trusted), "198.51.100.7");
    }

    #[test]
    fn trusted_peer_no_xff_returns_peer() {
        let trusted: Vec<IpNet> = vec!["127.0.0.0/8".parse().unwrap()];
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(client_ip(peer, &headers, &trusted), "127.0.0.1");
    }

    #[test]
    fn parse_trusted_proxies_skips_garbage() {
        let nets = parse_trusted_proxies("127.0.0.0/8, , garbage, 10.0.0.0/8");
        assert_eq!(nets.len(), 2);
    }
}

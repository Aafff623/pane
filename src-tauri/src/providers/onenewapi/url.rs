use reqwest::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedUrl {
    pub origin: String,
    pub hostname: String,
    pub http_plaintext: bool,
}

/// Trim, reject userinfo/query/fragment/non-root paths, and canonicalize to
/// `scheme://host[:port]` with no trailing slash.
pub fn normalize_base_url(raw: &str) -> Result<NormalizedUrl, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("URL is required".into());
    }
    let url = Url::parse(raw).map_err(|_| "invalid URL".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("only http and https URLs are supported".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL must not include a username or password".into());
    }
    if url.query().is_some() {
        return Err("URL must not include a query string".into());
    }
    if url.fragment().is_some() {
        return Err("URL must not include a fragment".into());
    }
    let path = url.path();
    if !matches!(path, "/" | "/v1" | "/v1/") {
        return Err("unsupported path — use the site origin, optionally /v1".into());
    }
    let hostname = url
        .host_str()
        .ok_or_else(|| "URL is missing a host".to_string())?
        .to_string();
    if hostname.is_empty() {
        return Err("URL is missing a host".into());
    }
    let origin = match url.port() {
        Some(port) => format!("{}://{}:{port}", url.scheme(), hostname),
        None => format!("{}://{}", url.scheme(), hostname),
    };
    Ok(NormalizedUrl {
        http_plaintext: url.scheme() == "http",
        origin,
        hostname,
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_base_url, NormalizedUrl};

    fn ok(input: &str, origin: &str, hostname: &str, http_plaintext: bool) {
        assert_eq!(
            normalize_base_url(input).expect(input),
            NormalizedUrl {
                origin: origin.into(),
                hostname: hostname.into(),
                http_plaintext,
            },
            "{input}"
        );
    }

    fn err(input: &str) {
        assert!(
            normalize_base_url(input).is_err(),
            "expected reject: {input}"
        );
    }

    #[test]
    fn accepts_https_host_and_canonical_paths() {
        ok(
            "https://api.example.com",
            "https://api.example.com",
            "api.example.com",
            false,
        );
        ok(
            "  https://api.example.com/  ",
            "https://api.example.com",
            "api.example.com",
            false,
        );
        ok(
            "https://api.example.com/v1",
            "https://api.example.com",
            "api.example.com",
            false,
        );
        ok(
            "https://api.example.com/v1/",
            "https://api.example.com",
            "api.example.com",
            false,
        );
        ok(
            "https://API.Example.COM:8443/v1/",
            "https://api.example.com:8443",
            "api.example.com",
            false,
        );
    }

    #[test]
    fn rejects_userinfo_query_fragment_and_other_paths() {
        err("https://user:pass@api.example.com");
        err("https://user@api.example.com/v1");
        err("https://api.example.com/?q=1");
        err("https://api.example.com?q=1");
        err("https://api.example.com/#frag");
        err("https://api.example.com/v1#frag");
        err("https://api.example.com/api");
        err("https://api.example.com/v1/models");
        err("https://api.example.com/v1/chat");
        err("ftp://api.example.com");
        err("");
        err("   ");
        err("not a url");
    }

    #[test]
    fn allows_public_http_and_https() {
        ok(
            "http://example.com",
            "http://example.com",
            "example.com",
            true,
        );
        ok("http://8.8.8.8", "http://8.8.8.8", "8.8.8.8", true);
        ok("http://172.15.0.1", "http://172.15.0.1", "172.15.0.1", true);
        ok("http://172.32.0.1", "http://172.32.0.1", "172.32.0.1", true);
        ok(
            "http://[2001:db8::1]",
            "http://[2001:db8::1]",
            "[2001:db8::1]",
            true,
        );
        ok(
            "https://example.com",
            "https://example.com",
            "example.com",
            false,
        );
        ok("https://8.8.8.8", "https://8.8.8.8", "8.8.8.8", false);
    }

    #[test]
    fn allows_localhost_loopback_private_and_local_http() {
        ok("http://localhost", "http://localhost", "localhost", true);
        ok("http://LOCALHOST/v1", "http://localhost", "localhost", true);
        ok("http://127.0.0.1", "http://127.0.0.1", "127.0.0.1", true);
        ok("http://[::1]/v1/", "http://[::1]", "[::1]", true);
        ok("http://10.1.2.3", "http://10.1.2.3", "10.1.2.3", true);
        ok("http://172.16.0.1", "http://172.16.0.1", "172.16.0.1", true);
        ok(
            "http://172.31.255.255:3000",
            "http://172.31.255.255:3000",
            "172.31.255.255",
            true,
        );
        ok(
            "http://192.168.1.9",
            "http://192.168.1.9",
            "192.168.1.9",
            true,
        );
        ok(
            "http://169.254.10.1",
            "http://169.254.10.1",
            "169.254.10.1",
            true,
        );
        ok("http://[fc00::1]", "http://[fc00::1]", "[fc00::1]", true);
        ok(
            "http://[fd12:3456:789a::1]",
            "http://[fd12:3456:789a::1]",
            "[fd12:3456:789a::1]",
            true,
        );
        ok("http://[fe80::1]", "http://[fe80::1]", "[fe80::1]", true);
        ok(
            "http://gateway.local",
            "http://gateway.local",
            "gateway.local",
            true,
        );
        ok(
            "http://PANEL.LOCAL/v1/",
            "http://panel.local",
            "panel.local",
            true,
        );
    }

    #[test]
    fn hostname_is_normalized_url_host() {
        let n = normalize_base_url("https://My-Panel.example.com/v1").unwrap();
        assert_eq!(n.hostname, "my-panel.example.com");
        assert!(!n.http_plaintext);
    }
}

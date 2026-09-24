//! Turning what the user typed into a site URL.

use anyhow::{Result, bail};
use url::Url;

/// Schemes that look like `name:rest` without `//` and must not be mistaken
/// for `host:port`.
const OPAQUE_SCHEMES: &[&str] = &[
    "about",
    "blob",
    "chrome",
    "data",
    "file",
    "javascript",
    "mailto",
    "tel",
    "view-source",
];

/// Parse a site URL. A bare host (`hey.com`) means `https://hey.com/`; only
/// `http` and `https` are accepted.
pub fn parse_site_url(input: &str) -> Result<Url> {
    let input = input.trim();
    if input.is_empty() {
        bail!("no URL given");
    }

    let candidate = if input.contains("://") {
        input.to_string()
    } else {
        if let Some((scheme, _)) = input.split_once(':')
            && OPAQUE_SCHEMES.contains(&scheme.to_ascii_lowercase().as_str())
        {
            bail!("unsupported URL scheme `{scheme}`: only http and https sites can be installed");
        }
        format!("https://{input}")
    };

    let url = Url::parse(&candidate).map_err(|e| anyhow::anyhow!("invalid URL `{input}`: {e}"))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            bail!("unsupported URL scheme `{other}`: only http and https sites can be installed")
        }
    }
    if url.host_str().is_none_or(str::is_empty) {
        bail!("invalid URL `{input}`: no host");
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_host_becomes_https() {
        assert_eq!(
            parse_site_url("hey.com").unwrap().as_str(),
            "https://hey.com/"
        );
    }

    #[test]
    fn bare_host_with_path_and_port() {
        assert_eq!(
            parse_site_url("localhost:8080/app").unwrap().as_str(),
            "https://localhost:8080/app"
        );
    }

    #[test]
    fn full_urls_are_normalised() {
        assert_eq!(parse_site_url("https://x/").unwrap().as_str(), "https://x/");
        assert_eq!(
            parse_site_url("  HTTP://GitHub.com  ").unwrap().as_str(),
            "http://github.com/"
        );
    }

    #[test]
    fn non_web_schemes_are_rejected_by_name() {
        for (input, scheme) in [
            ("file:///etc/passwd", "file"),
            ("ftp://example.com/", "ftp"),
            ("mailto:me@example.com", "mailto"),
            ("javascript:alert(1)", "javascript"),
        ] {
            let err = parse_site_url(input).unwrap_err().to_string();
            assert!(err.contains(&format!("`{scheme}`")), "{input}: {err}");
        }
    }

    #[test]
    fn empty_is_rejected() {
        assert!(parse_site_url("  ").is_err());
    }
}

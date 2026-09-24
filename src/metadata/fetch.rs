//! HTTP fetching with the limits from design decision 5.

use std::io::Read;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use reqwest::redirect::Policy;
use url::Url;

pub const TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_REDIRECTS: usize = 5;
pub const MAX_HTML: u64 = 2 * 1024 * 1024;
pub const MAX_MANIFEST: u64 = 1024 * 1024;
pub const MAX_ICON: u64 = 5 * 1024 * 1024;

/// Chromium's desktop User-Agent: some sites serve different markup, or no
/// manifest, to clients they don't recognise.
pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/153.0.0.0 Safari/537.36";

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("only http and https URLs can be fetched: {0}")]
    Scheme(Url),
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP status {0}")]
    Status(u16),
    #[error("response is larger than {limit} bytes")]
    TooLarge { limit: u64 },
    #[error("reading response: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Response {
    /// The URL after following redirects.
    pub final_url: Url,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

pub struct Fetcher {
    client: Client,
}

impl Fetcher {
    pub fn new() -> Result<Self, FetchError> {
        Self::with_timeout(TIMEOUT)
    }

    pub fn with_timeout(timeout: Duration) -> Result<Self, FetchError> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .redirect(Policy::limited(MAX_REDIRECTS))
            .build()?;
        Ok(Self { client })
    }

    /// GET `url`, failing on non-2xx statuses and bodies over `max_bytes`.
    pub fn get(&self, url: &Url, max_bytes: u64) -> Result<Response, FetchError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(FetchError::Scheme(url.clone()));
        }
        let response = self.client.get(url.clone()).send()?;
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::Status(status.as_u16()));
        }
        if response.content_length().is_some_and(|len| len > max_bytes) {
            return Err(FetchError::TooLarge { limit: max_bytes });
        }
        let final_url = response.url().clone();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v: &HeaderValue| v.to_str().ok())
            .map(str::to_ascii_lowercase);

        let mut body = Vec::new();
        response.take(max_bytes + 1).read_to_end(&mut body)?;
        if body.len() as u64 > max_bytes {
            return Err(FetchError::TooLarge { limit: max_bytes });
        }
        Ok(Response {
            final_url,
            content_type,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn url(server: &MockServer, path: &str) -> Url {
        Url::parse(&server.url(path)).unwrap()
    }

    #[test]
    fn follows_redirect_chain_and_reports_final_url() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/a");
            then.status(302).header("location", "/b");
        });
        server.mock(|when, then| {
            when.method(GET).path("/b");
            then.status(301).header("location", "/c");
        });
        server.mock(|when, then| {
            when.method(GET).path("/c").header("user-agent", USER_AGENT);
            then.status(200)
                .header("content-type", "text/html; charset=utf-8")
                .body("hello");
        });

        let resp = Fetcher::new().unwrap().get(&url(&server, "/a"), MAX_HTML).unwrap();
        assert_eq!(resp.final_url, url(&server, "/c"));
        assert_eq!(resp.content_type.as_deref(), Some("text/html; charset=utf-8"));
        assert_eq!(resp.body, b"hello");
    }

    #[test]
    fn too_many_redirects_fail() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/loop");
            then.status(302).header("location", "/loop");
        });
        let err = Fetcher::new().unwrap().get(&url(&server, "/loop"), MAX_HTML);
        assert!(matches!(err, Err(FetchError::Http(_))), "{err:?}");
    }

    #[test]
    fn timeout() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/slow");
            then.status(200).delay(Duration::from_millis(1500)).body("late");
        });
        let fetcher = Fetcher::with_timeout(Duration::from_millis(200)).unwrap();
        let err = fetcher.get(&url(&server, "/slow"), MAX_HTML).unwrap_err();
        match err {
            FetchError::Http(e) => assert!(e.is_timeout(), "{e}"),
            other => panic!("expected timeout, got {other:?}"),
        }
    }

    #[test]
    fn oversized_body_is_rejected() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/big");
            then.status(200).body(vec![b'x'; 2048]);
        });
        let err = Fetcher::new().unwrap().get(&url(&server, "/big"), 1024).unwrap_err();
        assert!(matches!(err, FetchError::TooLarge { limit: 1024 }), "{err:?}");
    }

    #[test]
    fn error_status_is_an_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/missing");
            then.status(404);
        });
        let err = Fetcher::new().unwrap().get(&url(&server, "/missing"), MAX_HTML).unwrap_err();
        assert!(matches!(err, FetchError::Status(404)), "{err:?}");
    }

    #[test]
    fn non_http_urls_are_refused() {
        let err = Fetcher::new()
            .unwrap()
            .get(&Url::parse("data:image/png;base64,AAAA").unwrap(), MAX_ICON)
            .unwrap_err();
        assert!(matches!(err, FetchError::Scheme(_)));
    }
}

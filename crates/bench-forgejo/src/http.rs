//! Minimal blocking HTTP GET built on skein's own client (no reqwest/tokio).
//!
//! Both callers are synchronous fixture entry points, so each request drives
//! skein's async [`HttpClient`] on a fresh one-shot current-thread runtime. The
//! runtime is built and dropped on the calling thread; callers that may run
//! inside another runtime already hop onto a plain OS thread first (see
//! `download::http_get`).

use skein::http::h1::http_client::{HttpClient, HttpClientConfig, DEFAULT_MAX_BODY_SIZE};
use skein::runtime::RuntimeBuilder;

/// A completed GET response: HTTP status and the full body bytes.
pub struct GetResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Perform a blocking GET, buffering up to `max_body_size` bytes of body.
///
/// Returns `Err(message)` on connect/protocol errors or an over-cap body.
pub fn blocking_get(url: &str, max_body_size: usize) -> Result<GetResponse, String> {
    let url = url.to_string();
    let runtime = RuntimeBuilder::current_thread()
        .build()
        .map_err(|err| format!("build http runtime: {err}"))?;
    runtime.block_on(async move {
        let client = HttpClient::with_config(HttpClientConfig {
            max_body_size,
            ..HttpClientConfig::default()
        });
        let response = client
            .get(&url)
            .await
            .map_err(|err| format!("GET {url}: {err}"))?;
        Ok(GetResponse {
            status: response.status,
            body: response.body,
        })
    })
}

/// Perform a blocking GET with skein's default body cap (16 MiB).
pub fn blocking_get_small(url: &str) -> Result<GetResponse, String> {
    blocking_get(url, DEFAULT_MAX_BODY_SIZE)
}

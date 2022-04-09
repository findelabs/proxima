use core::time::Duration;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper_tls::HttpsConnector;
use native_tls::TlsConnector;
use std::error::Error;

pub type HttpsClient = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;
type BoxResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub fn create_https_client(
    timeout: u64,
    set_nodelay: bool,
    enforce_http: bool,
    set_reuse_address: bool,
) -> BoxResult<HttpsClient> {
    // All this junk is needed to ensure that we can connect to an endpoint with bad certs/hostname
    let tls = TlsConnector::builder()
        .danger_accept_invalid_hostnames(true)
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut http = hyper::client::HttpConnector::new();

    // Create timeout Duration
    let timeout = Duration::new(timeout, 0);

    http.set_connect_timeout(Some(timeout));
    http.set_nodelay(set_nodelay);
    http.enforce_http(enforce_http);
    http.set_reuse_address(set_reuse_address);

    let https: hyper_tls::HttpsConnector<hyper::client::HttpConnector> =
        hyper_tls::HttpsConnector::from((http, tls.into()));
    Ok(hyper::Client::builder().build::<_, hyper::Body>(https))
}

use native_tls::TlsConnector;
use hyper::Body;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use std::error::Error;

pub type HttpsClient = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;
type BoxResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub fn create_https_client() -> BoxResult<HttpsClient> {
	// All this junk is needed to ensure that we can connect to an endpoint with bad certs/hostname
	let tls = TlsConnector::builder()
	    .danger_accept_invalid_hostnames(true)
	    .danger_accept_invalid_certs(true)
	    .build()?;
	
	let mut http = hyper::client::HttpConnector::new();
	http.enforce_http(false);
	let https: hyper_tls::HttpsConnector<hyper::client::HttpConnector> =
	    hyper_tls::HttpsConnector::from((http, tls.into()));
	Ok(hyper::Client::builder().build::<_, hyper::Body>(https))
}

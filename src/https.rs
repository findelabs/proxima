use core::time::Duration;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper::Request;
use hyper::Response;
use hyper_tls::HttpsConnector;
use native_tls::TlsConnector;
use std::error::Error;

//pub type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;
type BoxResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct HttpsClient(hyper::client::Client<HttpsConnector<HttpConnector>, Body>);

impl HttpsClient {
    pub async fn request(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let Self(internal) = self;
        internal.request(req).await
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    timeout: u64,
    set_nodelay: bool,
    enforce_http: bool,
    set_reuse_address: bool,
    accept_invalid_hostnames: bool,
    accept_invalid_certs: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    config: ClientConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            timeout: 60u64,
            set_nodelay: false,
            enforce_http: false,
            set_reuse_address: false,
            accept_invalid_hostnames: false,
            accept_invalid_certs: true,
        }
    }
}

impl Default for HttpsClient {
    fn default() -> Self {
        ClientBuilder::default().build().unwrap()
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        let config = ClientConfig::default();
        Self { config }
    }
    pub fn timeout(mut self, arg: u64) -> Self {
        self.config.timeout = arg;
        self
    }
    pub fn nodelay(mut self, arg: bool) -> Self {
        self.config.set_nodelay = arg;
        self
    }
    pub fn enforce_http(mut self, arg: bool) -> Self {
        self.config.enforce_http = arg;
        self
    }
    pub fn reuse_address(mut self, arg: bool) -> Self {
        self.config.set_reuse_address = arg;
        self
    }
    pub fn accept_invalid_hostnames(mut self, arg: bool) -> Self {
        self.config.accept_invalid_hostnames = arg;
        self
    }
    pub fn accept_invalid_certs(mut self, arg: bool) -> Self {
        self.config.accept_invalid_certs = arg;
        self
    }
    pub fn build(&mut self) -> BoxResult<HttpsClient> {
        let tls = TlsConnector::builder()
            .danger_accept_invalid_hostnames(self.config.accept_invalid_hostnames)
            .danger_accept_invalid_certs(self.config.accept_invalid_certs)
            .build()?;

        let mut http = hyper::client::HttpConnector::new();

        // Create timeout Duration
        let timeout = Duration::new(self.config.timeout, 0);

        http.set_connect_timeout(Some(timeout));
        http.set_nodelay(self.config.set_nodelay);
        http.enforce_http(self.config.enforce_http);
        http.set_reuse_address(self.config.set_reuse_address);

        let https: hyper_tls::HttpsConnector<hyper::client::HttpConnector> =
            hyper_tls::HttpsConnector::from((http, tls.into()));
        Ok(HttpsClient(
            hyper::Client::builder().build::<_, hyper::Body>(https),
        ))
    }
}

use core::time::Duration;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper::Request;
use hyper::Response;
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, TlsConnector};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config_global::GlobalConfig;
use crate::error::Error as ProximaError;

//pub type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;
//type BoxResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct HttpsClient(Arc<RwLock<hyper::client::Client<HttpsConnector<HttpConnector>, Body>>>);

impl HttpsClient {
    pub async fn request(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let locked = self.0.read().await;
        locked.request(req).await
    }

    // This should only be ran against a new HttpsClient
    pub async fn internal(self) -> hyper::client::Client<HttpsConnector<HttpConnector>, Body> {
        let rwlock = Arc::try_unwrap(self.0).expect("Unable to unwrap Arc");
        rwlock.into_inner()
    }

    pub async fn reconfigure(&mut self, global: &GlobalConfig) {
        match ClientBuilder::new()
            .timeout(global.network.timeout.value())
            .nodelay(global.network.nodelay)
            .enforce_http(global.network.enforce_http)
            .reuse_address(global.network.reuse_address)
            .accept_invalid_hostnames(global.security.tls.accept_invalid_hostnames)
            .accept_invalid_certs(global.security.tls.insecure)
            .import_cert(global.security.tls.import_cert.as_deref())
            .build()
        {
            Ok(c) => {
                log::debug!("\"Updating shared https client\"");
                let mut locked = self.0.write().await;
                let new = c.internal().await;
                *locked = new;
                log::debug!("\"Shared https client has been updated\"");
            }
            Err(e) => {
                log::error!("\"Error generating new client: {}\"", e);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfig<'a> {
    timeout: u64,
    set_nodelay: bool,
    enforce_http: bool,
    set_reuse_address: bool,
    accept_invalid_hostnames: bool,
    accept_invalid_certs: bool,
    import_cert: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder<'a> {
    config: ClientConfig<'a>,
}

impl Default for ClientConfig<'_> {
    fn default() -> Self {
        ClientConfig {
            timeout: 60u64,
            set_nodelay: false,
            enforce_http: false,
            set_reuse_address: false,
            accept_invalid_hostnames: false,
            accept_invalid_certs: true,
            import_cert: None,
        }
    }
}

impl Default for HttpsClient {
    fn default() -> Self {
        ClientBuilder::default().build().unwrap()
    }
}

impl<'a> ClientBuilder<'a> {
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
    pub fn import_cert(mut self, arg: Option<&'a str>) -> Self {
        self.config.import_cert = arg;
        self
    }
    pub fn build(&mut self) -> Result<HttpsClient, ProximaError> {
        let tls_connector = match self.config.import_cert {
            Some(path) => {
                let cert = &std::fs::read(path).expect("Failed reading in root cert");
                let import_cert =
                    Certificate::from_pem(cert).expect("Root cert is not in PEM format");
                log::info!("Reading in root cert at {}", &path);
                TlsConnector::builder()
                    .danger_accept_invalid_hostnames(self.config.accept_invalid_hostnames)
                    .danger_accept_invalid_certs(self.config.accept_invalid_certs)
                    .add_root_certificate(import_cert)
                    .build()?
            }
            None => TlsConnector::builder()
                .danger_accept_invalid_hostnames(self.config.accept_invalid_hostnames)
                .danger_accept_invalid_certs(self.config.accept_invalid_certs)
                .build()?,
        };

        let mut http = hyper::client::HttpConnector::new();

        // Create timeout Duration
        let timeout = Duration::new(self.config.timeout, 0);

        http.set_connect_timeout(Some(timeout));
        http.set_nodelay(self.config.set_nodelay);
        http.enforce_http(self.config.enforce_http);
        http.set_reuse_address(self.config.set_reuse_address);

        let https: hyper_tls::HttpsConnector<hyper::client::HttpConnector> =
            hyper_tls::HttpsConnector::from((http, tls_connector.into()));
        Ok(HttpsClient(Arc::new(RwLock::new(
            hyper::Client::builder().build::<_, hyper::Body>(https),
        ))))
    }
}

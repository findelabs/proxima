use axum::{
    handler::Handler,
    routing::{any},
    AddExtensionLayer, Router,
};
use chrono::Local;
use clap::{crate_version, App, Arg};
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{trace::TraceLayer};
use native_tls::TlsConnector;


mod handlers;
mod state;
mod config;

use handlers::{
    pass_through,
    handler_404
};

use state::{
    State,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let opts = App::new("rest-proxy-rs")
        .version(crate_version!())
        .author("Daniel F. <Verticaleap>")
        .about("rest-proxy-rs")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Set port to listen on")
                .required(false)
                .env("LISTEN_PORT")
                .default_value("8080")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .required(true)
                .help("Config file")
                .takes_value(true),
        )
        .get_matches();

    // Initialize log Builder
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{{\"date\": \"{}\", \"level\": \"{}\", \"log\": {}}}",
                Local::now().format("%Y-%m-%dT%H:%M:%S:%f"),
                record.level(),
                record.args()
            )
        })
        .target(Target::Stdout)
        .filter_level(LevelFilter::Error)
        .parse_default_env()
        .init();

    // Set port
    let port: u16 = opts.value_of("port").unwrap().parse().unwrap_or_else(|_| {
        eprintln!("specified port isn't in a valid range, setting to 8080");
        8080
    });

    
    // All this junk is needed to ensure that we can connect to an endpoint with bad certs/hostname
    let tls = TlsConnector::builder()
        .danger_accept_invalid_hostnames(true)
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut http = hyper::client::HttpConnector::new();
    http.enforce_http(false);
    let https: hyper_tls::HttpsConnector<hyper::client::HttpConnector> = hyper_tls::HttpsConnector::from((http, tls.into()));
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);
    let config_path = opts.value_of("config").unwrap().to_owned();
    let config = config::parse(&config_path)?;

    let state = Arc::new(State {
        config, 
        client: client
    });

    let base = Router::new()
        .route("/*path", any(pass_through));

    let app = Router::new()
        .merge(base)
        .layer(TraceLayer::new_for_http())
        .layer(AddExtensionLayer::new(state));

    // add a fallback service for handling routes to unknown paths
    let app = app.fallback(handler_404.into_service());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}



use axum::{
    handler::Handler,
    routing::{any, get, post},
    AddExtensionLayer, Router,
};
use chrono::Local;
use clap::{crate_version, App, Arg};
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::io::Write;
use std::net::SocketAddr;
use tower_http::auth::RequireAuthorizationLayer;
use tower_http::trace::TraceLayer;

mod config;
mod handlers;
mod https;
mod state;

use handlers::{config, echo, get_endpoint, handler_404, health, help, proxy, reload};
use https::create_https_client;
use state::State;

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
            Arg::with_name("timeout")
                .short("t")
                .long("timeout")
                .help("Set default global timeout")
                .required(false)
                .env("CONNECT_TIMEOUT")
                .default_value("60")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("username")
                .short("u")
                .long("username")
                .help("Set required username")
                .required(false)
                .env("AUTH_USERNAME")
                .requires("password")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("password")
                .short("p")
                .long("password")
                .help("Set required password")
                .required(false)
                .env("AUTH_PASSWORD")
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

    // Create state for axum
    let state = State::new(opts.clone()).await?;

    let base = Router::new()
        .route("/health", get(health))
        .route("/config", get(config))
        .route("/reload", post(reload))
        .route("/echo", post(echo))
        .route("/help", get(help))
        .route("/:endpoint", any(get_endpoint))
        .route("/:endpoint/*path", any(proxy));

    let app = match opts.is_present("username") {
        true => {
            let username = opts.value_of("username").expect("Missing username").to_string();
            let password = opts.value_of("password").expect("Missing username").to_string();
            Router::new()
                .merge(base)
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(state))
                .layer(RequireAuthorizationLayer::basic(&username, &password))
        }
        false => Router::new()
            .merge(base)
            .layer(TraceLayer::new_for_http())
            .layer(AddExtensionLayer::new(state)),
    };

    // add a fallback service for handling routes to unknown paths
    let app = app.fallback(handler_404.into_service());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

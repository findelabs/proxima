use axum::{
    extract::Extension,
    handler::Handler,
    middleware,
    routing::{any, get, post},
    Router,
};
use chrono::Local;
use clap::{crate_name, crate_version, Arg, Command};
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::io::Write;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod auth;
mod cache;
mod config;
mod error;
mod handlers;
mod https;
mod metrics;
mod path;
mod requests;
mod security;
mod state;
mod urls;
mod vault;
mod config_global;

use crate::metrics::{setup_metrics_recorder, track_metrics};
use handlers::{
    cache_delete, cache_get, config, echo, handler_404, health, help, mappings_get, metrics, proxy,
    reload, root,
};
use state::State;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let opts = Command::new(crate_name!())
        .version(crate_version!())
        .author("Daniel F. <Verticaleap>")
        .about(crate_name!())
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("Set port to listen on")
                .env("PROXIMA_LISTEN_PORT")
                .default_value("8080")
                .takes_value(true),
        )
        .arg(
            Arg::new("api_port")
                .short('P')
                .long("api_port")
                .help("Set API port to listen on")
                .env("PROXIMA_API_LISTEN_PORT")
                .default_value("8081")
                .takes_value(true),
        )
        .arg(
            Arg::new("config_username")
                .short('u')
                .long("config_username")
                .help("Set required username for config endpoint")
                .env("PROXIMA_AUTH_USERNAME")
                .requires("config_password")
                .takes_value(true),
        )
        .arg(
            Arg::new("config_password")
                .long("config_password")
                .help("Set required password for config endpoint")
                .env("PROXIMA_AUTH_PASSWORD")
                .requires("config_username")
                .takes_value(true),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .env("PROXIMA_CONFIG")
                .required(true)
                .help("Config file")
                .takes_value(true),
        )
        .arg(
            Arg::new("insecure")
                .long("insecure")
                .required(false)
                .help("Accept insecure https config")
                .takes_value(false),
        )
        .arg(
            Arg::new("vault_url")
                .long("vault_url")
                .required(false)
                .requires_all(&["vault_mount", "vault_login_path"])
                .env("VAULT_URL")
                .help("Vault url")
                .takes_value(true),
        )
        .arg(
            Arg::new("vault_kubernetes_role")
                .long("vault_kubernetes_role")
                .required(false)
                .requires("vault_url")
                .env("VAULT_KUBERNETES_ROLE")
                .help("Vault kubernetes role")
                .takes_value(true),
        )
        .arg(
            Arg::new("vault_role_id")
                .long("vault_role_id")
                .env("VAULT_ROLE_ID")
                .required(false)
                .requires("vault_secret_id")
                .help("Vault role_id")
                .takes_value(true),
        )
        .arg(
            Arg::new("vault_secret_id")
                .long("vault_secret_id")
                .env("VAULT_SECRET_ID")
                .required(false)
                .requires("vault_role_id")
                .help("Vault secret_id")
                .takes_value(true),
        )
        .arg(
            Arg::new("vault_mount")
                .long("vault_mount")
                .requires("vault_url")
                .env("VAULT_MOUNT")
                .help("Vault engine mount path")
                .takes_value(true),
        )
        .arg(
            Arg::new("vault_login_path")
                .long("vault_login_path")
                //                .default_value("auth/kubernetes")
                .requires("vault_url")
                .required(false)
                .env("VAULT_LOGIN_PATH")
                .help("Vault login path")
                .takes_value(true),
        )
        .arg(
            Arg::new("jwt_path")
                .long("jwt_path")
                .conflicts_with("vault_role_id")
                .default_value("/var/run/secrets/kubernetes.io/serviceaccount/token")
                .env("JWT_PATH")
                .help("JWT path"),
        )
        .get_matches();

    // Initialize log Builder
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
//                "{{\"date\": \"{}\", \"level\": \"{}\", \"module\": \"{}\", \"line\": \"{}\", \"log\": {}}}",
                "{{\"date\": \"{}\", \"level\": \"{}\", \"log\": {}}}",
                Local::now().format("%Y-%m-%dT%H:%M:%S:%f"),
                record.level(),
//                record.module_path().unwrap_or(""),
//                record.line().unwrap_or(0u32),
                record.args()
            )
        })
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    // Set main listen port
    let port: u16 = opts.value_of("port").unwrap().parse().unwrap_or_else(|_| {
        eprintln!("specified port isn't in a valid range, setting to 8080");
        8080
    });

    // Set API listen port
    let api_port: u16 = opts
        .value_of("api_port")
        .unwrap()
        .parse()
        .unwrap_or_else(|_| {
            eprintln!("specified API port isn't in a valid range, setting to 8081");
            8081
        });

    // Create state for axum, beginning with seed State which generates the shared HttpsClient
    let mut state = State::basic(opts.clone()).await;
    state.build(opts.clone()).await?;

    // Create prometheus handle
    let recorder_handle = setup_metrics_recorder();

    // API Routes
    let api = Router::new()
        .route("/config", get(config))
        .route("/reload", post(reload))
        .route("/cache", get(cache_get).delete(cache_delete))
        .route("/mappings", get(mappings_get))
        .route("/health", get(health))
        .route("/echo", post(echo))
        .route("/help", get(help))
        .route("/metrics", get(metrics))
        .layer(TraceLayer::new_for_http())
        .route_layer(middleware::from_fn(track_metrics))
        .layer(Extension(state.clone()))
        .layer(Extension(recorder_handle.clone()));

    let app = Router::new()
        .route("/", any(root))
        .route("/:endpoint", any(proxy))
        .route("/:endpoint/*path", any(proxy))
        .layer(TraceLayer::new_for_http())
        .route_layer(middleware::from_fn(track_metrics))
        .layer(Extension(state))
        .layer(Extension(recorder_handle));

    // add a fallback service for handling routes to unknown paths
    let proxy = app.fallback(handler_404.into_service());
    let api = api.fallback(handler_404.into_service());

    // Create server for main proxy
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("\"Proxy listening on {}\"", addr);
    let server1 =
        axum::Server::bind(&addr).serve(proxy.into_make_service_with_connect_info::<SocketAddr>());

    // Create server for API
    let addr = SocketAddr::from(([0, 0, 0, 0], api_port));
    log::info!("\"API listening on {}\"", addr);
    let server2 =
        axum::Server::bind(&addr).serve(api.into_make_service_with_connect_info::<SocketAddr>());

    tokio::try_join!(server1, server2)?;

    Ok(())
}

use axum::{
    extract::Extension,
    handler::Handler,
    middleware,
    routing::{any, delete, get, post},
    Router,
};
use chrono::Local;
use clap::{crate_name, crate_version, App, Arg};
use env_logger::{Builder, Target};
use log::LevelFilter;
use std::io::Write;
use std::net::SocketAddr;
use tower_http::auth::RequireAuthorizationLayer;
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
mod state;
mod urls;
mod vault;

use crate::metrics::{setup_metrics_recorder, track_metrics};
use handlers::{
    clear_cache, config, echo, get_cache, handler_404, health, help, metrics, proxy, reload,
    remove_cache, root,
};
use state::State;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let opts = App::new(crate_name!())
        .version(crate_version!())
        .author("Daniel F. <Verticaleap>")
        .about(crate_name!())
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Set port to listen on")
                .env("PROXIMA_LISTEN_PORT")
                .default_value("8080")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("timeout")
                .short("t")
                .long("timeout")
                .help("Set default global timeout")
                .default_value("60")
                .env("PROXIMA_TIMEOUT")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("username")
                .short("u")
                .long("username")
                .help("Set required client username")
                .env("PROXIMA_CLIENT_USERNAME")
                .requires("password")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("password")
                .short("p")
                .long("password")
                .help("Set required client password")
                .requires("username")
                .env("PROXIMA_CLIENT_PASSWORD")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config_username")
                .short("u")
                .long("config_username")
                .help("Set required username for config endpoint")
                .env("PROXIMA_AUTH_USERNAME")
                .requires("config_password")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config_password")
                .short("p")
                .long("config_password")
                .help("Set required password for config endpoint")
                .env("PROXIMA_AUTH_PASSWORD")
                .requires("config_username")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .env("PROXIMA_CONFIG")
                .required(true)
                .help("Config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("set_nodelay")
                .long("nodelay")
                .required(false)
                .help("Set socket nodelay")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("enforce_http")
                .long("enforce_http")
                .required(false)
                .help("Enforce http protocol for remote endpoints")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("set_reuse_address")
                .long("reuse_address")
                .required(false)
                .help("Enable socket reuse")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("accept_invalid_hostnames")
                .long("accept_invalid_hostnames")
                .required(false)
                .help("Accept invalid remote hostnames")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("accept_invalid_certs")
                .long("accept_invalid_certs")
                .required(false)
                .help("Accept invalid remote certificates")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("vault_url")
                .short("v")
                .long("vault_url")
                .required(false)
                .env("VAULT_URL")
                .help("Vault url")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("vault_kubernetes_role")
                .short("r")
                .long("vault_kubernetes_role")
                .required(false)
                .requires("vault_url")
                .env("VAULT_KUBERNETES_ROLE")
                .help("Vault kubernetes role")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("vault_role_id")
                .short("i")
                .long("role_id")
                .env("VAULT_ROLE_ID")
                .required(false)
                .requires("vault_secret_id")
                .help("Vault role_id")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("vault_secret_id")
                .short("S")
                .long("secret_id")
                .env("VAULT_SECRET_ID")
                .required(false)
                .requires("vault_role_id")
                .help("Vault secret_id")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("vault_mount")
                .short("m")
                .long("vault_mount")
                .required(false)
                .requires("vault_url")
                .env("VAULT_MOUNT")
                .help("Vault engine mount path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("vault_login_path")
                .short("l")
                .long("vault_login_path")
                .default_value("auth/kubernetes")
                .required(false)
                .env("VAULT_LOGIN_PATH")
                .help("Vault login path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("jwt_path")
                .short("j")
                .long("jwt_path")
                .default_value("/var/run/secrets/kubernetes.io/serviceaccount/token")
                .required(false)
                .env("JWT_PATH")
                .help("JWT path")
                .takes_value(true),
        )
        .get_matches();

    // Initialize log Builder
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{{\"date\": \"{}\", \"level\": \"{}\", \"module\": \"{}\", \"line\": \"{}\", \"log\": {}}}",
                Local::now().format("%Y-%m-%dT%H:%M:%S:%f"),
                record.level(),
                record.module_path().unwrap_or(""),
                record.line().unwrap_or(0u32),
                record.args()
            )
        })
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    // Set port
    let port: u16 = opts.value_of("port").unwrap().parse().unwrap_or_else(|_| {
        eprintln!("specified port isn't in a valid range, setting to 8080");
        8080
    });

    // Create state for axum
    let state = State::default().build(opts.clone()).await?;

    // Create prometheus handle
    let recorder_handle = setup_metrics_recorder();

    // These should be authenticated
    let closed = Router::new()
        .route("/-/config", get(config))
        .route("/-/reload", post(reload))
        .route("/-/cache", get(get_cache).delete(clear_cache))
        .route("/-/cache/*entry", delete(remove_cache))
        .route("/:endpoint", any(proxy))
        .route("/:endpoint/*path", any(proxy));

    // These should NOT be authenticated
    let open = Router::new()
        .route("/", get(root))
        .route("/-/health", get(health))
        .route("/-/echo", post(echo))
        .route("/-/help", get(help))
        .route("/-/metrics", get(metrics));

    let app = match opts.is_present("username") {
        true => {
            let username = opts
                .value_of("username")
                .expect("Missing username")
                .to_string();
            let password = opts
                .value_of("password")
                .expect("Missing username")
                .to_string();
            Router::new()
                .merge(closed)
                .layer(RequireAuthorizationLayer::basic(&username, &password))
                .merge(open)
                .layer(TraceLayer::new_for_http())
                .route_layer(middleware::from_fn(track_metrics))
                .layer(Extension(state))
                .layer(Extension(recorder_handle))
        }
        false => Router::new()
            .merge(closed)
            .merge(open)
            .layer(TraceLayer::new_for_http())
            .route_layer(middleware::from_fn(track_metrics))
            .layer(Extension(state))
            .layer(Extension(recorder_handle)),
    };

    // add a fallback service for handling routes to unknown paths
    let app = app.fallback(handler_404.into_service());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("\"Listening on {}\"", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

# Proxima

Ultra fast, simple, http proxy.

### What is Proxima?

Proxima is a simple L7 proxy, commonly used as an API gateway, acting as a single entry point for your microservices. Proxima supports connecting to backend endpoints over http, or https. View the docs [here](https://findelabs.github.io/proxima/installation.html)!

### How do I configure Proxima?

Proxima is configured via a simple yaml file, which specifies all routes and subroutes that Proxima will serve. Additionally, you can point Proxima endpoints at remote http sites from which to pull dynamic endpoints. You can also read secrets from Hashicorp Vault, formatting the secrets with handlebars templates.

A Proxima endpoint is great at handling client security, to include authentication and method whitelisting. The security field allows for multiple types of client authentication on a single endpoint, including basic, digest, bearer, and JWT via JWKS.

Each endpoint can also authenticate users against the remote URL specified. This is great for masking authentication to remote API's requiring API keys. Using Proxima, you can use a single API key to authenticate to a remote endpoint, yet still requiring unique credentials for internal clients. An example of this is below:

```
remote_endpoint:
  url: http://localhost:8081/endpoint
  
  # Specify creds for remote url
  authentication:
    basic:
      username: imkcdads
      password: s.cdanjfiewionkacnklcdaslcds

  # Require client authentication
  security:
    client:
      basic:
      - username: admin
        password: admin
      - username: client_one
        password: passwd_one
      - username: client_two
        password: passwd_two
```

More security options are shown under the examples directory.

### Proxima Usage
```
proxima 0.7.1
Daniel F. <Verticaleap>
proxima

USAGE:
    proxima [OPTIONS] --config <config>

OPTIONS:
        --accept_invalid_certs        Accept invalid remote certificates
        --accept_invalid_hostnames    Accept invalid remote hostnames
    -c, --config                      Config file [env: PROXIMA_CONFIG=] 
        --config_password             Set required password for config endpoint [env: PROXIMA_AUTH_PASSWORD=]
        --enforce_http                Enforce http protocol for remote endpoints
    -h, --help                        Print help information
        --import_cert                 Import CA certificate [env: PROXIMA_IMPORT_CERT=]
        --jwt_path                    JWT path [env: JWT_PATH=]
        --nodelay                     Set socket nodelay
    -p, --port                        Set port to listen on [env: PROXIMA_LISTEN_PORT=] [default: 8080]
    -P, --api_port                    Set API port to listen on [env: PROXIMA_API_LISTEN_PORT=] [default: 8081]
        --password                    Set required client password [env: PROXIMA_CLIENT_PASSWORD=]
        --reuse_address               Enable socket reuse
    -t, --timeout                     Set default global timeout [env: PROXIMA_TIMEOUT=] [default: 60]
    -u, --config_username             Set required username for config endpoint [env: PROXIMA_AUTH_USERNAME=]
        --username                    Set required client username [env: PROXIMA_CLIENT_USERNAME=]
    -V, --version                     Print version information
        --vault_kubernetes_role       Vault kubernetes role [env: VAULT_KUBERNETES_ROLE=]
        --vault_login_path            Vault login path [env: VAULT_LOGIN_PATH=]
        --vault_mount                 Vault engine mount path [env: VAULT_MOUNT=]
        --vault_role_id               Vault role_id [env: VAULT_ROLE_ID=]
        --vault_secret_id             Vault secret_id [env: VAULT_SECRET_ID=]
        --vault_url                   Vault url [env: VAULT_URL=]
```

### Promima Metrics

Proxima exposes a prometheus metrics endpoint by default, at /-/metrics. The following metrics are exposed:

- TYPE proxima_cache_attempt_total counter  
- TYPE proxima_cache_keys gauge  
- TYPE proxima_cache_miss_total counter  
- TYPE proxima_config_renew_attempts_total counter  
- TYPE proxima_config_renew_failures_total counter  
- TYPE proxima_jwts_renew_attempts_total counter  
- TYPE proxima_jwts_renew_failures_total counter  
- TYPE proxima_requests_duration_seconds histogram  
- TYPE proxima_requests_total counter  
- TYPE proxima_response_errors_total counter  
- TYPE proxima_security_client_authentication_total counter  
- TYPE proxima_security_method_attempts_total counter  
- TYPE proxima_security_method_blocked_total counter  


### Proxima API Endpoints

Proxima exposes a series of endpoints you may hit, listed below:
```
"/-/cache":
  methods:
    delete: Delete proxima cache
    get: Get proxima cache
"/-/config":
  methods:
    get: Get proxima configuration
"/-/echo":
  methods:
    get: Echo back json payload (debugging)
"/-/health":
  methods:
    get: Get the health of the api
"/-/help":
  methods:
    get: Show this help message
"/-/reload":
  methods:
    get: Reload the api's config
"/:endpoint":
  methods:
    get: Show config for specific parent
"/:endpoint/*path":
  methods:
    get: Pass through any request to specified endpoint

```

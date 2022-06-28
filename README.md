# Proxima

Ultra fast, simple, http proxy.

### What is Proxima
Proxima is a simple L7 proxy, commonly used as an API gateway, acting as a single entry point for your microservices. Proxima supports connecting to backend endpoints over http, or https. View the docs [here](https://findelabs.github.io/proxima/installation.html)!

### How do I configure Proxima

Currently, you configure the endpoints behind proxima using either a static configuration file, or by pointing Proxima at a http endpoint, where the config can be scraped. 

Proxima is great at handling endpoint authentication for users. If a remote url requires authentication, simply specify basic, digest, or token authentication fields for the url. Basic auth would look something like this:
```
remote_endpoint:
  url: http://localhost:8081/endpoint
  authentication:
    basic:
      username: admin
      password: testing
```

Proxima can also lock down specific endpoints using basic, digest, or token authentication. This configuration looks exactly like the authentication settings shown above, however, we specify lock, not authentication. The example below will require any client hitting proxima to use digest authentication:
```
locked_endpoint:
  url: http://localhost:8081/endpoint
  security:
    client:
      digest:
      - username: admin
        password: testing
```

There are example configs under examples, but the general shape should look something like:

```
static_config:
  # This remote endpoint requires basic authentication, which will be handled by proxima
  endpoint_requiring_basic:
    url: http://localhost:8080
    timeout: 5000   # Timeout after 5 seconds
    authentication:
      basic:
        username: user
        password: testing

  # This remote endpoint requires digest authentication
  endpoint_requiring_digest:
    url: http://localhost:8080
    authentication:
      digest:
        username: user
        password: testing

  # This endpoint requires token authentication
  endpoint_requiring_bearer:
    url: http://localhost:8080
    authentication:
      bearer:
        token: dGhpc2lzdGhlYmVzdHRlc3Rpbmd0b2tlbmV2ZXJ0aGFua3lvdXZlcnltdWNoCg==

  # This is a multi-level set of endpoints
  level_two:
    endpoint_one:
      url: http://localhost:8080
    endpoint_two:
      url: http://localhost:8080
    level_three:
      endpoint_four:
        url: http://localhost:8080
```

You can also pull a config from a remote https endpoint by specifying a url with the --config flag.

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

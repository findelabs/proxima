# Proxima

Ultra fast, simple, http proxy.

### What is Proxima
Proxima is a simple L7 proxy, used for stitching together multiple http endpoints behind a single endpoint. Proxima supports connecting to backend endpoints over http, or https. View the docs [here](https://findelabs.github.io/proxima/installation.html)!

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

Proxima can also lock down specific endpoints using basic, digest, or token authentication. This configuration looks exactly like the authentication settings shown above. However, specific lock, not authentication. The example below will require any client hitting proxima to use digest authentication:
```
locked_endpoint:
  url: http://localhost:8081/endpoint
    lock:
      digest:
        username: admin
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
proxima 0.5.27
Daniel F. <Verticaleap>
proxima

USAGE:
    proxima [FLAGS] [OPTIONS] --config <config>

FLAGS:
        --enforce_http     Enforce http protocol for remote endpoints
    -h, --help             Prints help information
        --nodelay          Set socket nodelay
        --reuse_address    Enable socket reuse
    -V, --version          Prints version information

OPTIONS:
    -c, --config <config>                      Config file [env: PROXIMA_CONFIG=]
    -p, --config_password <config_password>    Set required password for config endpoint [env: PROXIMA_AUTH_PASSWORD=]
    -u, --config_username <config_username>    Set required username for config endpoint [env: PROXIMA_AUTH_USERNAME=]
    -p, --password <password>                  Set required client password [env: PROXIMA_CLIENT_PASSWORD=]
    -p, --port <port>                          Set port to listen on [env: PROXIMA_LISTEN_PORT=]  [default: 8080]
    -t, --timeout <timeout>                    Set default global timeout [env: PROXIMA_TIMEOUT=]  [default: 60]
    -u, --username <username>                  Set required client username [env: PROXIMA_CLIENT_USERNAME=]
```

### Promima Metrics

Proxima exposes a prometheus metrics endpoint by default, at /-/metrics. The following metrics are exposed:

- TYPE proxima_cache_attempts_total counter  
- TYPE proxima_cache_hit_total counter  
- TYPE proxima_config_renew_attempts_total counter  
- TYPE proxima_config_renew_success_total counter  
- TYPE proxima_endpoint_authentication_total counter  
- TYPE proxima_endpoint_authentication_basic_failed_total counter  
- TYPE proxima_endpoint_authentication_digest_failed_total counter  
- TYPE proxima_endpoint_authentication_token_failed_total counter  
- TYPE proxima_requests_duration_seconds histogram  
- TYPE proxima_requests_total counter  


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

# Proxima

Ultra fast, simple, http proxy.

### What is Proxima
Proxima is a simple L7 proxy, and is used for stitching together multiple http endpoints behind a single proxima proxy. Proxima supports connecting to backend endpoints over http, or https.

### How do I configure Proxima
Currently, you configure the endpoints behind proxima using either a static configuration file, or by pointing Proxima at a http endpoint, where the config can be scraped. There are example configs under examples, but the general shape should look something like:

```
static_config:
  endpoint_requiring_basic:
    url: http://localhost:8080
    authentication:
      basic:
        username: user
        password: testing
  endpoint_requiring_bearer:
    url: http://localhost:8080
    authentication:
      bearer:
        token: dGhpc2lzdGhlYmVzdHRlc3Rpbmd0b2tlbmV2ZXJ0aGFua3lvdXZlcnltdWNoCg==
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
USAGE:
    proxima [OPTIONS] --config <config>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <config>                      Config file [env: PROXIMA_CONFIG=]
    -p, --config_password <config_password>    Set required password for config endpoint [env: PROXIMA_AUTH_PASSWORD=]
    -u, --config_username <config_username>    Set required username for config endpoint [env: PROXIMA_AUTH_USERNAME=]
    -p, --password <password>                  Set required client password [env: PROXIMA_CLIENT_PASSWORD=]
    -p, --port <port>                          Set port to listen on [env: PROXIMA_LISTEN_PORT=]  [default: 8080]
    -t, --timeout <timeout>                    Set default global timeout [env: PROXIMA_TIMEOUT=]  [default: 60]
    -u, --username <username>                  Set required client username [env: PROXIMA_CLIENT_USERNAME=]
```

### Proxima API Endpoints

Proxima exposes a series of endpoints you may hit, listed below:
```
{
  "paths": {
    "/-/config": "Get config of api",
    "/-/echo": "Echo back json payload (debugging)",
    "/-/health": "Get the health of the api",
    "/-/help": "Show this help message",
    "/-/reload": "Reload the api's config",
    "/:endpoint": "Show config for specific parent",
    "/:endpoint/*path": "Pass through any request to specified endpoint"
  }
}
```

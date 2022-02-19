# Proxima

Ultra fast, simple, http proxy.

### What is Proxima
Proxima is a simple L7 proxy, and is used for stitching together multiple http endpoints behind a single proxima proxy. Proxima supports connecting to backend endpoints over http, or https.

### How do I configure Proxima
Currently, you configure the endpoints behind proxima using either a static configuration file, or by pointing Proxima at a http endpoint, where the config can be scraped. There are example configs under examples, but the general shape should look something like:

```
endpoint_one:
  url: https://google.com
endpoint_two:
  url: https://yahoo.com
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

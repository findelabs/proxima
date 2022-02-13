# Proxima

Ultra fast, simple, http proxy

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

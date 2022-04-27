# Command Line Arguments

## Quick Start
Basic usage to start Proxima is 
```bash
proxima --config config.yaml
```

## Flags

Flags are optional to start Proxima.

#### --enforce_http
Using `--enforce_http` causes Proxima to error if any endpoints specify non-http urls.

#### --help
Print out help information with either `--help` or `-h`.

#### --nodelay
Enable socket nodelay with `--nodelay`, read about this [here](https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux_for_real_time/7/html/tuning_guide/tcp_nodelay_and_small_buffer_writes).

#### --reuse_address
Proxima will reuse socket if possible when `--reuse_address` is specified.

#### --accept_invalid_hostnames
Ignore hostnames that do not match the request

#### --accept_invalid_certs
Accept certs that are not valid on remote servers

#### --version
Print Proxima version with `--version` or `-v`


## Options

#### --config [env: PROXIMA_CONFIG]
Specify a yaml config file with `--config` or `-c`. This configuration can either be a file, or an http endpoint.

#### --config_username [env: PROXIMA_AUTH_USERNAME]
If you config file is an http endpoint that requires authentication, specify a username with `--config_username`.

#### --config_password [env: PROXIMA_AUTH_PASSWORD]
If you config file is an http endpoint that requires authentication, specify a password with `--config_password`.

#### --port [env: PROXIMA_LISTEN_PORT]
Set the port on which to listen with `--port` or `-p`, default of 8080.

#### --username [env: PROXIMA_CLIENT_USERNAME]
Force all clients hitting Proxima to authenticate with Basic creds, with `--username` or `-u` specifying the username.

#### -password [env: PROXIMA_CLIENT_PASSWORD]
Force all clients hitting Proxima to authenticate with Basic creds, with `--password` or `-p` specifying the password.

#### --timeout [env: PROXIMA_TIMEOUT]
Set a global connection timeout with `--timeout`, default is 60 seconds.

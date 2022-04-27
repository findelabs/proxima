# Authentication

Proxima can both connect to remote endpoints using the following authentication methods, and authenticate client connections to Proxima in order to verify user identities:
- Basic  
- Bearer  
- Digest

### Client Authentication

If you would like to require that all incoming clients to Proxima authentication with Basic creds, simply create a new lock block within the endpoint yaml. Please keep in mind that this lock map will not appear when listing the config via the Proxima REST API:

```yaml
static_config:
  endpoint_locked:
    url: http://myurl.net
    lock:
      basic:
        username: client
        password: mypassword
```

### Remote URL Authentication

If a remote endpoint requires authentication, for example Basic, simply specify a new authentication block within the endpoint yaml:

```yaml
static_config:
  endpoint_basic:
    url: http://myurl.net
    authentication:
      basic:
        username: myusername
        password: mypassword
```

### Authentication Configuration

Proxima currently supports Basic and Digest for username/password authentication, as well as Bearer token authentication. 

Here are some examples on how to specify each of these authentication types, for an endpoint, remembering that each endpoint can only support one:
```yaml
static_config:
  endpoint_test:
    url: http://myurl.net
    authentication:
      basic:                  # Authenticate with remote endpoint with Basic
        username: client
        password: mypassword
      digest:                 # Authenticate with remote endpoint with Digest
        username: client
        password: mypassword
      token:                  # Authenticate with remote endpoint with Token
        token: Y2Rhc2Nkc2NkYXNjc2QK



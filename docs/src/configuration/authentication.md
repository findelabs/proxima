# Authentication

Proxima can both connect to remote endpoints using the following authentication methods:

- Basic  
- Bearer  
- Digest

Proxima can authenticate users using all authentication methods listed above, but it can also authenticate users JWT's as well using JWKS. This authentication can be specified with `jwks`.


### Simple Client Authentication

If you would like to require that all incoming clients to Proxima authentication with Basic creds, simply create a new lock block within the endpoint yaml. Please keep in mind that this lock map will not appear when listing the config via the Proxima REST API:

```yaml
static_config:
  endpoint_locked:
    url: http://myurl.net
    security:
      client:
        basic:
        - username: client_one
          password: mypasswordone
        - username: client_two
          password: mypasswordtwo
```

#### Client JWKS Configuration

Proxima can authenticate users' JWT's by caching the JWKS from other authentication providers, like Okta. 

Here is an example on how to configure an endpoint with jwks client authentication:

```yaml
static_config:
  endpoint_test:
    url: http://myurl.net
    security:
      client:
        jwks:
        - url: https://dev-17129172.okta.com/oauth2/default/v1/keys
          audience: api://default
          scopes:
          - findelabs.test
```

With this configuration, after a user generates a token via the Okta /token endpoint, include said token field in the Authorization header of the request to Proxima.

### Remote Server Authentication

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

Proxima currently supports Basic and Digest for username/password authentication, as well as Bearer token authentication. 

Here are some examples on how to specify each of these authentication types, for an endpoint. Keep in mind that the authentication block only supports one type of authentication for an endpoint:

```yaml
static_config:
  endpoint_test:
    url: http://myurl.net
    authentication:
      basic:                  # Authenticate with remote endpoint with Basic or
        username: client
        password: mypassword
      # or
      digest:                 # Authenticate with remote endpoint with Digest or
        username: client
        password: mypassword
      # or
      token:                  # Authenticate with remote endpoint with Token
        token: Y2Rhc2Nkc2NkYXNjc2QK



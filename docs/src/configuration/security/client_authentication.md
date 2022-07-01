# Client Authentication

Proxima is able to authenticate clients using any of the following protocols:

- Basic  
- Bearer  
- Digest
- JWKS

### Basic and Digest Client Authentication

If you would like to require that one user logs in with Basic, and another with Digest, simply create a new security.client map that contains both basic and digest, within the endpoint yaml. 

```yaml
routes:
  locked:
    url: http://myurl.net
    security:
      client:
        basic:
        - username: client_one
          password: mypasswordone
        digest:
        - username: client_digest
          password: mypasswordtwo
```

#### Client JWKS Configuration

Proxima can authenticate users' JWT's by caching the JWKS from other authentication providers, like Okta. 

Here is an example on how to configure an endpoint with jwks client authentication:

```yaml
routes:
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


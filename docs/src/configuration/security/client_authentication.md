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
    proxy:
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
    proxy:
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

#### Client Bearer Configuration

Proxima can authenticate user's token as well, by using the token client security type.

Here is an example on how to configure an endpoint with bearer client authentication:

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
      security:
        client:
          bearer:
          - token: s.YWQ5MWY2N2RiMTE1ZjNhZDdkOTFiOGZl
```

#### Client API Key Configuration

Proxima can also authenticate users based on a specified header's value. The default header name is set to `x-api-key`, but this name can be set to any arbitrary value.

Here is an example on how to configure an endpoint with api_key client authentication:

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
      security:
        client:
          api_key:
          - token: s.YWQ5MWY2N2RiMTE1ZjNhZDdkOTFiOGZl
          - token: s.NDQ4YzIzNzA0OWI4YmU1MjVkN2M4ZDZi
            key: api-key
```


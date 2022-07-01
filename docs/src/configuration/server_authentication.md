# Remote URL Authentication

Proxima can both connect to remote endpoints using the following authentication methods:

- Basic  
- Bearer  
- Digest

If a remote endpoint requires authentication, for example, Basic authentication, simply specify a new authentication field within the endpoint yaml:

```yaml
routes:
  endpoint_requiring_basic:
    url: http://myurl.net
    authentication:
      basic:
        username: myusername
        password: mypassword
```

Proxima currently supports Basic and Digest for username/password authentication, as well as Bearer token authentication. 

Here are some examples on how to specify each of these authentication types, for an endpoint. Keep in mind that the authentication block only supports one type of authentication for an endpoint:

```yaml
routes:
  endpoint_test:
    url: http://myurl.net
    authentication:
      basic:                  # Authenticate with remote endpoint with Basic or
      - username: client
        password: mypassword
      # or
      digest:                 # Authenticate with remote endpoint with Digest or
      - username: client
        password: mypassword
      # or
      bearer:                 # Authenticate with remote endpoint with Token
      - token: Y2Rhc2Nkc2NkYXNjc2QK
```


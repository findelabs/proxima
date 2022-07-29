# Remote Server Authentication

Proxima can both connect to remote URL's using the following authentication methods:

- Basic  
- Bearer  
- Digest
- API Key
- JWT

If a remote endpoint requires authentication, for example, Basic authentication, simply specify a new authentication field within the endpoint yaml:

```yaml
routes:
  endpoint_requiring_basic:
    proxy:
      url: http://myurl.net
      authentication:
        basic:
          username: myusername
          password: mypassword
```

Here are some examples on how to specify each of these authentication types, for an endpoint. Keep in mind that the authentication block only supports one type of authentication for an endpoint:

#### Basic

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
    authentication:
      basic:
        username: client
        password: mypassword
```

#### Digest

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
    authentication:
      digest:
        username: client
        password: mypassword
```

#### Bearer

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
    authentication:
      bearer:
        token: s.Y2Rhc2Nkc2NkYXNjc2QK
```

#### API Key

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
    authentication:
      api_key:
        token: s.Y2Rhc2Nkc2NkYXNjc2QK
        key: api-key
```

#### JWT

```yaml
routes:
  endpoint_test:
    proxy:
      url: http://myurl.net
    authentication:
      jwt:
        url: https://dev-8177876213.okta.com/oauth2/default/v1/token
        audience: 0oa4cdaknkn866cdacd
        client_id: njcda8981cds
        client_secret: s.Y2Rhc2Nkc2NkYXNjc2QK
        grant_type: client_credentials
        scopes: 
        - default
```


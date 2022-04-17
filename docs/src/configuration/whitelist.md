# Whitelist

Proxima is currently able to whitelist specific methods for endpoints. The following methods can currently be whitelisted:

- Options
- Get
- Post
- Put
- Delete
- Head
- Trace
- Connect
- Patch

### Method Whitelisting


If a remote endpoint requires authentication, for example Basic, simply specify a new authentication block within the endpoint yaml:

```yaml
static_config:
  endpoint_basic:
    url: http://myurl.net
    whitelist:
      methods:
      - GET
      - POST
```

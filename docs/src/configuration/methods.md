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

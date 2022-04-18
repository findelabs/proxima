### Method Whitelisting

You can whitelist specific methods for an endpoint, as shown below:

```yaml
static_config:
  endpoint_basic:
    url: http://myurl.net
    whitelist:
      methods:
      - GET
      - POST
```

The following methods can currently be whitelisted:

- Options
- Get
- Post
- Put
- Delete
- Head
- Trace
- Connect
- Patch

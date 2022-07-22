# Proxy

This endpoint variant will forward all requests that terminate the config entry. For example, with the following config:

```yaml
routes:
  api:
    proxy:
      url: http://localhost:8081
```

A user request to localhost:8080/api/health will return back the response from http://localhost:8081/health.

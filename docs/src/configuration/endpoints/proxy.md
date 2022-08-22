# Proxy

This endpoint variant will forward all requests that terminate the config entry. For example, with the following config:

```yaml
routes:
  api:
    proxy:
      url: http://localhost:8081
```

A user request to localhost:8080/api/health will return back the response from http://localhost:8081/health.

### Proxy Endpoint Details

| Name                                        | Description                                         | Value      |
|-------------------------------------------- | --------------------------------------------------- | ---------- |
| proxy.url                                   | URL for remote server                               | `""`       |
| proxy.authentication                        | Enable the sending of credentials to remote server  | `{}`       |
| proxy.timeout                               | Endpoint timeout after connection is established    | `u64`      |
| proxy.security.client                       | Enable client authentication                        | `{}`       |
| proxy.security.whitelist.networks           | Enable network whitelisting                         | `[]`       |
| proxy.security.whitelist.methods            | Enable method authentication                        | `[]`       |
| proxy.config.preserve_host_header           | Retain original client HOST header                  | `{}`       |

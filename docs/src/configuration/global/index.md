# Global Config

Example global config is shown below:

```yaml
global:
  network:
    enforce_http: Bool
    nodelay: Bool
    reuse_address: Bool
    timeout: u64
  security:
    config:
      hide_folders: Bool
    tls:
      accept_invalid_hostnames: Bool
      insecure: Bool
      import_cert: String
    auth:
      client:
        api_key:
        basic:
        bearer:
        digest:
        jwks:
      whitelist:
        networks: Vec<CIDR>
        methods: Vec<Methods>
```

### Config Item Details

| Name                                         | Description                                         | Value         |
|--------------------------------------------- | --------------------------------------------------- | ------------- |
| global.network.enforce_http                  | Enforce http-type endpoints                         | `false`       |
| global.network.nodelay                       | Enable TCP nodelay on packets                       | `false`       |
| global.network.reuse_address                 | Reuse sockets when establishing connections         | `false`       |
| global.network.timeout                       | Set global connection timeout                       | `false`       |
| global.security.config.hide_folders          | Return 404 for non-endpoints (folders)              | `false`       |
| global.security.tls.accept_invalid_hostnames | Accept invalid hostnames when using https           | `false`       |
| global.security.tls.insecure                 | Accept incorrect certs when using https             | `false`       |
| global.security.tls.import_cert              | Specify cert to import                              | `""`          |
| global.security.auth.client                  | Set default client auth (overridden at endpoint)    | `{}`          |
| global.security.auth.whitelist.networks      | Set default network whitelist                       | `[]`          |
| global.security.auth.whitelist.methods       | Set default method whitelist                        | `[]`          |


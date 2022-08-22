# Static

This endpoint variant simply returns back a body to the client. This can easily be used to override the default root page of Proxima. 

An example override of `/` shown below:

```yaml
routes:
  /:
    static:
      body: "hello world"
```

### Static Endpoint Details

| Name                                        | Description                                         | Value      |
|-------------------------------------------- | --------------------------------------------------- | ---------- |
| static.body                                  | Body of response                                    | `""`       |
| static.security.client                       | Enable client authentication                        | `{}`       |
| static.security.whitelist.networks           | Enable network whitelisting                         | `[]`       |
| static.security.whitelist.methods            | Enable method authentication                        | `[]`       |
| static.headers                               | Add headers to response                             | `{}`       |

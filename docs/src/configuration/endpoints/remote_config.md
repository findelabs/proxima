# Remote Config

This variant will pull the sub-config from a remote url. This can be used to stitch multiple Proximas together, in order to share a config. 

An example of this is shown below:

```yaml
routes:
  proxima:
    http_config:
      url: http://localhost:8081/config
```



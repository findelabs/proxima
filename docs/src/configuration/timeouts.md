# Endpoint Timeouts

Proxima has a default connection timeout of 60 seconds that can be overridden with a command line argument or environmental variable. However, there is also a request timeout that is set at the individual endpoint level, also with a default of 60 seconds.

If 60 seconds is considered too long for your remote endpoint, you can override the setting like the following (in ms):
```yaml
static_config:
  my_endpoint:
    url: http://google.com
    timeout: 5000
```

# Syntax

The configuration file of Proxima always starts with a `static_config` block, under which all remote endpoints reside. Proxima can handle multi-level endpoints as well. 

For example, to have Proxima send any clients requesting `/host/yahoo` to yahoo.com, and `/host/google` to google.com, your config.yaml would look something like:
```yaml
static_config:
  host:
    yahoo:
      url: http://yahoo.com
    google:
      url: http:://google.com
```

### Request Timeouts

Proxima has a default connection timeout of 60 seconds that can be overridden with a command line argument or environmental variable. However, there is also a request timeout that is set at the individual endpoint level, also with a default of 60 seconds. 

If 60 seconds is considered too long for your remote endpoint, you can override the setting like the following (in ms):
```yaml
static_config:
  my_endpoint:
    url: http://google.com
    timeout: 5000
``` 

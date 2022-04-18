# URL Failover

Proxima allow for URL's to be failed over when a remote URL socket is not open, or when the url times out. 

Configure an endpoint like below in order to have the URL failover. In this example, if google.com failes to respond, then duckduckgo.com will be promoted as the primary URL for this endpoint.
```yaml
static_config:
  search:
    url: 
      failover:
      - http://google.com
      - http://duckduckgo.com
```

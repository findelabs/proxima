# Endpoint Security

Proxima allows for the whitelisting of both methods and networks. An example of an endpoint that allows GET and POST requests, from private networks, is shown below:

```yaml
routes:
  endpoint_basic:
    url: http://myurl.net
    security:
      whitelist:
        methods:
        - GET
        - POST
        networks:
        - 10.0.0.0/8
        - 127.0.0.0/16
        - 192.168.0.0/24
```

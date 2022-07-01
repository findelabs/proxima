# Method Whitelisting

You can whitelist specific methods for all clients for an endpoint.

```yaml
routes:
  endpoint_basic:
    url: http://myurl.net
    security:
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


### Client Whitelists

You can also whitelist specific methods for specific clients under security.clients[].whitelist, as shown below. Keep in mind that clients will only be able to use methods included in the global endpoint configuration.

```yaml
routes:
  endpoint_basic:
    url: http://myurl.net
    security:
      whitelist:
        methods:
        - GET
        - POST
      client:
        basic:
        
        # This will work
        - username: myuser_one
          password: mypassword
          whitelist:
            methods:
            - GET
            
        # This will fail since PUT is not a globally whitelisted endpoint method
        - username: myuser_two
          password: mypassword
          whitelist:
            methods:
            - PUT
```


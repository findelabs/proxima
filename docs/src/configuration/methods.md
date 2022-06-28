### Endpoint Method Whitelisting

You can globally whitelist specific methods for an endpoint, as shown below:

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


### Whitelisting Client Methods

You can also whitelist specific methods for specific clients under security.clients[].whitelist, as shown below. Keep in mind that if you also have specified a list of globally whitelisted methods for the endpoint, the clients will only be able to access a subset of those methods.

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
            
        # This will fail
        - username: myuser_two
          password: mypassword
          whitelist:
            methods:
            - PUT
```


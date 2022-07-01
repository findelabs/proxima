# Network Whitelisting

Proxima is able to whitelist CIDR networks. You may configure both global whitelisted endpoint networks, as well per-client whitelisted networks. The global network whitelist trumps client whitelists, in that if a client network is not included in the global whitelisted networks, connections from the client network will be refused.

The global endpoint network whitelist is not a requirement. You keep network whitelisted solely at the client whitelist level.

An example of a series of whitelisted networks with a global whitelist is shown below:

```yaml
routes:
  single:
    url: http://localhost:3000
    security:
      whitelist:
        networks:
        - 192.168.1.0/24
        - 192.168.0.0/24
      client:
        basic:
        # This admin user will only be allowed to authenticate from a single network
        - username: admin1
          password: adminpassword1
          whitelist:
            networks:
            - 192.168.0.0/24
        # This admin user will only be allowed to authenticate from a single network
        - username: admin2
          password: adminpassword2
          whitelist:
            networks:
            - 192.168.1.0/24
```

Here is an example of an endpoint that does not specify a global network whitelist:

```yaml
routes:
  single:
    url: http://localhost:3000
    security:
      client:
        basic:
        - username: admin1
          password: adminpassword1
          whitelist:
            networks:
            - 192.168.0.0/24
```

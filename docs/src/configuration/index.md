# Configuration

Proxima is configured via a yaml config file, which currently specifies just the subfolders, endpoints, and various endpoint options which Proxima will serve. 

In the future, we will be including many more configuration options within this config.

### Configuration Syntax

The configuration file of Proxima always starts with a `routes` block, under which all remote endpoints reside. Proxima can handle multi-level endpoints as well.

For example, to have Proxima send any clients requesting `/host/yahoo` to yahoo.com, and `/host/google` to google.com, your config.yaml would look something like:
```yaml
routes:
  host:
    proxy:
      yahoo:
        url: http://yahoo.com
      google:
        url: http:://google.com
```

Alternatively, Proxima can also serve redirects for various sites, as shown in the example below:
```yaml
routes:
  links:
    google: 
      redirect:
        url: https://google.com
    craigslist: 
      redirect:
        url: https://craigslist.com
```

### Optional Endpoint Fields

Proxy and static endpoint variants can contain a series of optional fields:

- Endpoint timeouts (timeout)
- Security (security)
  - Client Authentication (security.client)  
  - Method Whitelisting  (security.whitelist.method)
  - Network Whitelisting  (security.whitelist.method)
- Remote URL Authentication  (authentication)

### Dynamic Loading

Proxima will check for changes to the config file every 30 seconds by default. If the newest changes are unparsable, Proxima will continue to operate using the previous working configuration.


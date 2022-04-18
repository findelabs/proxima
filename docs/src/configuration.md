# Configuration

Proxima is configured via a yaml config file, which currently specifies just the subfolders, endpoints, and various endpoint options which Proxima will serve. 

In the future, we will be including many more configration options within this config.

### Configuration Syntax

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

### Optional Endpoint Fields

Each endpoint block can contain a series of optional fields:

- Endpoint timeouts (timeout)
- Client Authentication (lock)
- Remote URL Authentication
- Method Whitelist

### Dynamic Loading

Proxima will check for changes to the config file every 30 seconds by default. If the newest changes are unparsable, Proxima will continue to operate using the previous working configuration.

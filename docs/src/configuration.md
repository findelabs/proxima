# Configuration

Proxima is configured via a yaml config file, which currently specifies just the subfolders, endpoints, and various endpoint options which Proxima will serve. 

In the future, we will be including many more configration options within this config.

### Dynamic Loading
Proxima will check for changes to the config file every 30 seconds by default. If the newest changes are unparsable, Proxima will continue to operate using the previous working configuration.

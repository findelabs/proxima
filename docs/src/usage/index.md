# Proxima Usage

Proxima is, at heart, a layer seven proxy, and is configured using a yaml config file, and also allows for multi-level routing structures.

For example, a simple configuration could look like the following. With this config, with proxima running on localhost on port 8080, a GET to `localhost:8080/google` will get redirected to https://www.google.com.

```yaml
routes:
  google:
    url: https://www.google.com
  yahoo:
    url: https://yahoo.com
```


A more complex configuration could also proxy in the following. With this config, with a GET to `/user1/search`, the call will get sent to google, whereas a GET to `/user2/search` will get forwarded to yahoo.

```yaml
routes:
  user1:
    search:
      url: https://www.google.com
  user2:
    search:
      url: https://yahoo.com
```

# Redirect

With this variant, hitting this endpoint will cause proxima to return a 308 Permanent Redirect to the client, along with the next hop location in the headers.

Example of such config below:

```yaml
routes:
  google:
    redirect:
      url: https://google.com
```

### Redirect Endpoint Details

| Name                                        | Description                                         | Value      |
|-------------------------------------------- | --------------------------------------------------- | ---------- |
| redirect.url                                   | URL for remote server                            | `""`       |

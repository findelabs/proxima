# Static

This endpoint variant simply returns back a body to the client. This can easily be used to override the default root page of Proxima. 

An example override of `/` shown below:

```yaml
routes:
  /:
    static:
      body: "hello world"
```

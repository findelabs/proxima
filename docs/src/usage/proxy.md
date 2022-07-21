# Proxying Requests

The main use case for Proxima is to handle clients hitting various endpoints specified in the config. General usage is below:

## List Sub Endpoints
View all endpoints under a path

**URL** : `/:endpoint`

**Method** : `GET`

**Sample Response**

```json
{
  "endpoint_four": {
    "url": "http://localhost:8080/"
  },
  "endpoint_three": {
    "url": "http://localhost:8080/"
  },
  "level_three": {
    "endpoint_five": {
      "url": "http://localhost:8080/"
    }
  }
}
```
---
## Endpoint Forward
Endpoints can forward, redirect, and display static content:

**URL** : `/:endpoint/*path`

**Method** : `ANY`

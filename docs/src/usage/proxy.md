# Proxy

The main use case for Proxima is to handle the proxying of clients to remote urls. Usage is below:

## View Endpoint
View all endpoints under a path

**URL** : `/:endpoint`

**Method** : `GET`

**Auth required** : No

#### Success Response:

**Code** : `200 OK`

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
## Endpoint Pass Through
Proxy request to remote endpoint

**URL** : `/:endpoint/*path`

**Method** : `ANY`

**Auth required** : Maybe

#### Success Response:

**Code** : `200 OK`

# REST API

Proxima exposes a series of admin API paths under `/-/`:

## Get Cache
Get the current url cache hashmap.

**URL** : `/-/cache`

**Method** : `GET`

**Auth required** : No

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "archivelabs": "https://api.archivelab.org/v1/",
  "archivelabs/arcade": "https://api.archivelab.org/v1/arcade"
}
```
---
## Delete Cache or Entry
Get the current url cache hashmap.

**URL** : `/-/cache[/key]`

**Method** : `DELETE`

**Auth required** : No

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "entry": "archivelabs",
  "msg": "entry remove from cache"
}
```
---
## Show Config
Get Proxima's current config

**URL** : `/-/config`

**Method** : `GET`

**Auth required** : No

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "static_config": {
    "archivelabs": {
      "timeout": 5000,
      "url": "https://api.archivelab.org/v1"
    },
    "inshortsapi": {
      "url": "https://inshortsapi.vercel.app/news"
    },
    "local": {
      "url": "http://localhost:8082/"
    }
  }
}
```
---
## Show Health
Get Proxima's current health

**URL** : `/-/health`

**Method** : `GET`

**Auth required** : No

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "msg": "Healthy"
}
```
---
## Reload Config
Reload Proxima Config

**URL** : `/-/reload`

**Method** : `POST`

**Auth required** : No

#### Success Response:

**Code** : `200 OK`


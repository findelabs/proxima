# REST API

Proxima exposes a series of admin API paths on a secondary port, by default 8081.

## Show Config
Get Proxima's current config

**URL** : `/config`

**Method** : `GET`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "routes": {
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

**URL** : `/health`

**Method** : `GET`

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

**URL** : `/reload`

**Method** : `POST`

#### Success Response:

**Code** : `200 OK`

---
## Get Mappings Cache
Get the current mappings cache.

**URL** : `/mappings`

**Method** : `GET`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "/archivelabs": "archivelabs",
  "/inshortsapi": "inshortsapi"
}
```
---
## Get Cache
Get the current url cache.

**URL** : `/cache`

**Method** : `GET`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "archivelabs": "https://api.archivelab.org/v1",
  "inshortsapi": "https://inshortsapi.vercel.app/news"
}
```
---
## Delete Cache
Delete Proxima's cache

**URL** : `/cache`

**Method** : `DELETE`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "msg": "cache has been cleared"
}
```
---
## Delete Cache Entry
Get the current url cache hashmap.

**URL** : `/cache?key=[key]`

**Method** : `DELETE`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "entry": "archivelabs",
  "msg": "entry remove from cache"
}
```

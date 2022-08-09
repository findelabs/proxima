# REST API

Proxima exposes a series of admin API paths on a secondary port, by default 8081.

## Show Config
Get Proxima's current configuration

**URL** : `/config`

**Method** : `GET`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "network": {
    "enforce_http": false,
    "nodelay": false,
    "reuse_address": false,
    "timeout": 5000
  },
  "security": {
    "config": {
      "hide_folders": false
    },
    "tls": {
      "accept_invalid_hostnames": false,
      "insecure": false
    },
    "whitelist": {
      "networks": [
        "10.0.0.0/8"
      ]
    }
  }
}
```
---
## Show Routes
Get Proxima's current routes

**URL** : `/routes`

**Method** : `GET`

#### Success Response:

**Code** : `200 OK`

**Sample Response**

```json
{
  "routes": {
    "archivelabs": {
      "proxy": {
        "timeout": 5000,
        "url": "https://api.archivelab.org/v1"
      }
    },
    "inshortsapi": {
      "proxy": {
        "url": "https://inshortsapi.vercel.app/news"
      }
    },
    "local": {
      "proxy": {
        "url": "http://localhost:8082/"
      }
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

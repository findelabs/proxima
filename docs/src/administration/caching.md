# Endpoint Caching

Proxima employs a two-stage cache; the first which maps a Proxima config folder path to config endpoint, and the second which maps unique client request paths to the first stage cache. 

#### First Stage
The first stage cache can be viewed by hitting the `/cache` endpoint on the admin port, and may show contents like:
```
{
  "archivelabs": "https://api.archivelab.org/v1",
  "inshortsapi": "https://inshortsapi.vercel.app/news"
}
```
What this shows is that there are at least two endpoints listed within Proxima's config, one for archivelabs, and the other for inshortsapi. The value for both of these keys is simply displayed as the remote URL, but within the cache is the entire endpoint, which includes any authentication, or security parameters. 

#### Second Stage
The second stage cache can be accessed by hitting the `/mappings` endpoint, also served on the admin port. Example contents corresponding to the first stage cache listed above may look something like:

```
{
  "/archivelabs": "archivelabs",
  "/inshortsapi": "inshortsapi",
  "/inshortsapi/foobar": "inshortsapi"
}
```

What this means is that at least one client has hit Proxima at the paths /archivelabs, /inshortsapi, and /inshortsapi/foobar. If another client were to request another unique path from Proxima which matches a known endpoint, that too would be added to the second stage cache.

#### Relearning Endpoints

If the endpoints for any of the entries in the cache were learned from a remote source, such as Vault, and then had their Vault secret modified, the first stage cache will have invalid data. This is acknowledged as a potential risk, and we plan on introducing a cache timeout in the future. 

For now, if a Vault secret is modified that has already been cached, you may remove the cached entry by using a delete method against `/cache?key=inshortsapi`, which in this case will remove the entry for inshortsapi from the first stage cache. There is no delete option available for the second stage cache, since the likelyhood of a mapping going bad is exceedingly low. 

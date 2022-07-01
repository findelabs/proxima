# Hashicorp Vault Integration

Proxima is able to pull dynamic secrets from Hashicorp Vault, and can cache these remote endpoints locally in the endpoint cache. This reduces the necassary calls to Vault to just a single GET, with subsequent proxy requests being served from memory.

Currently Proxima can be directed to pull secrets from a single Vault endpoint, and can be configured to authenticate via approle, or JWT. An example of the flags for both methods are shown below:

#### Vault AppRole Authentication
```
./proxima --config $CONFIG \
--vault_role_id $ROLE_ID \
--vault_secret_id $SECRET_ID \
--vault_url https://vault.local:8200 \
--vault_mount kv2
```

##### Vault JWT Authentication (within pod)
```
./proxima --vault_kubernetes_role kubernetes \
--vault_login_path auth/kubernetes \
--vault_url https://vault.local:8200 \
--vault_mount kv2
```

### Endpoint Configuration

Proxima will attempt to authenticate to the Vault upon startup. Proxima will only begin to serve endpoints once it is able to connect to Vault succesfully. Next, we will need to ensure that the endpoints are pointed at Vault secret folders. This is done by specifying both the secret folder, and an optional template, in case of a more complex folder of secrets. Below is an example of how one could setup an endpoint designed to use Digest authentication to connect to the MongoDB Atlas REST API.

```
routes:
  atlas:
    template: eyJ1cmwiOiJ7eyB1cmwgfX0iLCJhdXRoZW50aWNhdGlvbiI6eyJkaWdlc3QiOnsidXNlcm5hbWUiOiJ7eyB1c2VybmFtZSB9fSIsInBhc3N3b3JkIjoie3sgcGFzc3dvcmQgfX0ifX19Cg==
    secret: atlas/apis/
```


When we unpack the example template, we find: `{"url":"{{ url }}","authentication":{"digest":{"username":"{{ username }}","password":"{{ password }}"}}}`

The Vault folder found under `kv2/atlas/apis` can contain any number of secrets, as long as they contain a minimum number of fields specified in the base64-encoded template: url, username, and password. Proxima will attempt to apply the template against any secrets found within `kv2/atlas/apis`. 

For example, if a running Proxima were configured with this config, a GET request to `http://localhost:8080/atlas/rest` will cause Proxima to attempt to pull a secret in Vault at `kv2/atlas/apis/rest`, and will attempt to apply the template against the contents. If the resulting json can be successfully parsed as an endpoint, Proxima will route the client to the remote url with digest authentication. If the secret does not contain the fields specified within the template, Proxima will log an error, and will return a 404 status code.


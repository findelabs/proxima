# Hashicorp Vault Integration

Proxima can be directed to pull secrets from a single Vault endpoint, and can be configured to authenticate via approle, or JWT. Proxima is able to source either an entire secret directory from Vault, or a single secret. 

Proxima is able to pull dynamic secrets from Hashicorp Vault, and can cache these remote endpoints locally in the endpoint cache. This reduces the necassary calls to Vault to just a single GET, with subsequent proxy requests being served from memory.

Examples on how to configure Proxima to connect to Vault is shown below:

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

Proxima will attempt to authenticate to the Vault upon startup. Proxima will only begin to serve endpoints once it is able to connect to Vault succesfully. Next, we will need to ensure that the endpoints are pointed at a secret or folder that exists. This is done by specifying both the secret path, and an optional template, in case of a complex secrets. Below is an example of how one could setup an endpoint designed to use Digest authentication to connect to the MongoDB Atlas REST API.

```
routes:
  atlas:
    vault:
      template: eyJ1cmwiOiJ7eyB1cmwgfX0iLCJhdXRoZW50aWNhdGlvbiI6eyJkaWdlc3QiOnsidXNlcm5hbWUiOiJ7eyB1c2VybmFtZSB9fSIsInBhc3N3b3JkIjoie3sgcGFzc3dvcmQgfX0ifX19Cg==
      secret: atlas/apis/rest
```

When we unpack the example template, we find: `{"url":"{{ url }}","authentication":{"digest":{"username":"{{ username }}","password":"{{ password }}"}}}`

The Vault secret found at `kv2/atlas/apis/rest` can contain any number of fields,  as long as they contain a minimum number of fields specified in the base64-encoded template: url, username, and password. 

### Vault Secret Folder

Proxima is also able to grab secrets from a folder within  Vault. Proxima differentiate a folder vs a single secret by whether or not the secret provided has a trailing forward slash.

For example, if a running Proxima were configured with an endpoint named atlas, with a Vault secret pointing at `/atlas/apis/`, a GET request to `http://localhost:8080/atlas/rest` will cause Proxima to attempt to pull a secret in Vault at `kv2/atlas/apis/rest`, and will attempt to apply the template against the contents. If the resulting json can be successfully parsed as an endpoint, Proxima will route the client to the remote url with digest authentication. If the secret does not contain the fields specified within the template, Proxima will log an error, and will return a 404 status code.

Additionally, a GET request to `http://localhost:8080/atlas` will cause Proxima to attempt to apply the template against any secrets found within `kv2/atlas/apis`, returning back a json payload with any endpoints it found within the folder. 

### Vault Endpoint Details

| Name                                        | Description                                         | Value      |
|-------------------------------------------- | --------------------------------------------------- | ---------- |
| vault.secret                                | Path to secret in Vault                             | `""`       |
| vault.template                              | Template to use when rendering secret               | `""`       |

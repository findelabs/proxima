# Endpoints

Below is a list of the endpoint variations that Proxima supports, along with a brief description:

#### Remote Config

This variant will attempt to pull all sub folders from the url specified. 

#### Proxy

This variant will forward request payload, method, and headers, along with sub folders, to the specified url.

#### Redirect

This variant will simply redirect all requests to the specified url.

#### Static

Specify a static body to serve.

#### Vault

Connect to a HashiCorp Vault instance, in order to pull dynamic secrets, optionally templated to match any matching variant using handlebars.

# Overview

Proxima is a simple L7 proxy, commonly used as an API gateway, acting as a single entry point for your microservices. Proxima supports connecting to backend endpoints over http/https, and can source dynamic endpoints through http calls, or through Hashicorp Vault.

# Docker

You can pull down Proxima images at [docker](https://hub.docker.com/repository/docker/findelabs/proxima).

# Compile

You can compile Proxima by running `cargo build --release` after pulling down the [repo](https://github.com/findelabs/proxima).

Alternatively, you may run `cargo install --git https://github.com/findelabs/proxima.git`.

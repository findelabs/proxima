# Docker

Findelabs pushes new images to [docker.com](https://hub.docker.com/r/findelabs/proxima/tags). 

You will need to provide Proxima with a basic configuration file in order to start. Below is a bare-bones config:
```bash
cat << EOF > config.yaml
routes:
  endpoint:
    url: https://google.com
EOF
```

You can then start a Proxima container on a server running docker with something like:
```bash
docker run -p 8080:8080 \
--mount type=bind,source="$(pwd)"/config.yaml,target=/config.yaml\
findelabs/proxima:latest --config /config.yaml
```

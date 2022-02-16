from rust:slim as builder

RUN mkdir /app 
RUN mkdir /app/bin 

COPY src /app/src/
COPY Cargo.toml /app

RUN apt-get update && apt-get install -y libssl-dev pkg-config
RUN cargo install --path /app --root /app
RUN strip app/bin/proxima

FROM debian:bullseye-slim
WORKDIR /app
COPY --from=builder /app/bin/ ./

ENTRYPOINT ["/app/rust-api-template"]
EXPOSE 8080

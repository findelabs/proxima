from rust:slim as builder

RUN mkdir /app 
RUN mkdir /app/bin 

COPY src /app/src/
COPY Cargo.toml /app

RUN apt-get update && apt-get install -y libssl-dev pkg-config
RUN cargo install --path /app --root /app

from rust:slim
WORKDIR /app
COPY --from=builder /app/bin/ ./

ENTRYPOINT ["/app/rest-proxy-rs"]
EXPOSE 8080

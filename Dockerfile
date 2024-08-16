FROM debian:bookworm-slim
RUN mkdir /app
COPY target/binrelease/server /app/server
COPY target/site /app/target/site
COPY Cargo.toml /app/Cargo.toml

WORKDIR /app
ENTRYPOINT ["/app/server"]
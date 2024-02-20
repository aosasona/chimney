#------- Build the Rust binary -------#
FROM rust:1.76 AS builder

# Use "aarch64" if you are on M* Mac - --build-arg ARCH="aarch64"
ARG ARCH="x86_64"

WORKDIR /source

COPY ./Cargo.toml ./Cargo.toml

COPY ./Cargo.lock ./Cargo.lock

COPY ./src ./src

RUN rustup target add $ARCH-unknown-linux-musl

RUN cargo build --target=$ARCH-unknown-linux-musl --release

#------- Copy into run image -------#
FROM alpine:3.19.1

ARG ARCH

COPY --from=builder /source/target/${ARCH}-unknown-linux-musl/release/chimney /bin/chimney

# Create the default "public" directory follownig the normal convention
RUN mkdir -p /var/www/html

# Create default config
RUN mkdir -p /etc/chimney
RUN echo $'host = "0.0.0.0" \n\
port = 80 \n\
enable_logging = true \n\
root_dir = "/var/www/html" \n\
fallback_document = "index.html"' > /etc/chimney/chimney.toml

# Create default index page
RUN echo $'<h1>Hello, World!</h1>\n\
<p>If you can see this page, you have successfully setup and started Chimney.</p>\n\
<p>Copy your own config file to <b>`/chimney.toml`</b> and your static files to the <b>`/var/www/html`</b> directory (unless you changed it) to serve your files.</p>' > /var/www/html/index.html

ENV PATH="/bin/chimney:$PATH"

ENTRYPOINT ["chimney"]

CMD ["run"]

LABEL org.opencontainers.image.title="Chimney"

LABEL org.opencontainers.image.description="A simple, fast, and easy to use static file server."

LABEL org.opencontainers.image.authors="Ayodeji O. <ayodeji@trulyao.dev>"

LABEL org.opencontainers.image.source="https://github.com/aosasona/chimney"


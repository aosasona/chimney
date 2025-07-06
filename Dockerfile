#------- Build the Rust binary -------#
FROM rust:1.88 AS builder

# Use "aarch64" if you are on M* Mac - --build-arg ARCH="aarch64"
ARG ARCH="x86_64"

WORKDIR /source

COPY ./Cargo.toml ./Cargo.toml

COPY ./Cargo.lock ./Cargo.lock

COPY ./crates ./crates

RUN rustup target add $ARCH-unknown-linux-musl

RUN cargo build --target=$ARCH-unknown-linux-musl --release

#------- Copy into run image -------#
FROM alpine:3.22.0

ARG ARCH

COPY --from=builder /source/target/${ARCH}-unknown-linux-musl/release/chimney-cli /bin/chimney

# Create the default "public" directory follownig the normal convention
RUN mkdir -p /var/www/html

# Create default config
RUN mkdir -p /etc/chimney
RUN echo $'host = "0.0.0.0" \n\
	port = 80 \n\
	host_detection = "auto" \n\
	sites_directory = "/var/www/html" \n\
	log_level = "trace"' > /etc/chimney/chimney.toml

# Create default index page
RUN mkdir -p /var/www/html/default
RUN echo $'root = "." \n\
	domain_names = ["*"] \n\
	fallback_file = "404.html" \n\
	default_index_file = "index.html"' > /var/www/html/default/chimney.toml
RUN echo $'<h1>Hello, World!</h1>\n\
	<p>If you can see this page, you have successfully setup and started Chimney.</p>\n\
	<p>Copy your own config file to <b>`/etc/chimney/chimney.toml`</b> and your static files to the <b>`/var/www/html/default`</b> directory (unless you changed it) to serve your files.</p>' > /var/www/html/default/index.html

ENV PATH="/bin/chimney:$PATH"

ENTRYPOINT ["chimney"]

CMD ["serve"]

LABEL org.opencontainers.image.title="Chimney"

LABEL org.opencontainers.image.description="A tiny, fast, and modern static file server."

LABEL org.opencontainers.image.authors="Ayodeji O. <ayodeji@trulyao.dev>"

LABEL org.opencontainers.image.source="https://github.com/aosasona/chimney"

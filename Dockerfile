#------- Build the Rust binary -------#
FROM rust:1.88-slim AS builder

WORKDIR /source

COPY ./Cargo.toml ./Cargo.toml

COPY ./Cargo.lock ./Cargo.lock

COPY ./crates ./crates

RUN cargo build --release

#------- Copy into run image -------#
FROM debian:bookworm-slim

COPY --from=builder /source/target/release/chimney-cli /bin/chimney

# Create the default "public" directory follownig the normal convention
RUN mkdir -p /var/www/html

# Create default config
RUN mkdir -p /etc/chimney
RUN echo $'host = "0.0.0.0" \n\
	port = 80 \n\
	host_detection = "auto" \n\
	sites_directory = "/var/www/html" \n\
	log_level = "info"' > /etc/chimney/config.toml

# Create default index page
RUN mkdir -p /var/www/html/default
RUN echo $'root = "." \n\
	domain_names = ["*"] \n\
	fallback_file = "404.html" \n\
	default_index_file = "index.html"' > /var/www/html/default/chimney.toml
RUN echo $'<h1>Hello, World!</h1>\n\
	<p>If you can see this page, you have successfully setup and started Chimney.</p>\n\
	<p>Copy your own config file to <b>`/etc/chimney/config.toml`</b> and your static files to the <b>`/var/www/html/default`</b> directory (unless you changed it) to serve your files.</p>' > /var/www/html/default/index.html

ENV PATH="/bin/chimney:$PATH"

ENTRYPOINT ["chimney"]

CMD ["serve"]

LABEL org.opencontainers.image.title="Chimney"

LABEL org.opencontainers.image.description="A tiny, fast, and modern static file server."

LABEL org.opencontainers.image.authors="Ayodeji O. <ayodeji@trulyao.dev>"

LABEL org.opencontainers.image.source="https://github.com/aosasona/chimney"

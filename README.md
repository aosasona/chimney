> [!WARNING]
> This is still in early development, I would not recommend for production use.. yet.
>
> See the [issues](https://github.com/aosasona/chimney/issues) for things that are on the "roadmap" or missing
>
> This may not fit your usecase, have a look at [Nginx](https://www.nginx.com/) and [Caddy](https://caddyserver.com/)

A tiny static file server. See [this example](https://trulyao.fly.dev) deployed on [Fly](https://fly.io).

# Goals

- **As tiny as possible**
- **Reasonably fast**
- Serve files with the correct mime-types
- Predictable "routing" (the way you will expect it from like Nginx or Apache eg. if `/foo` is a folder, it should resolve fine to `/foo/index.html`)
- Rewrites and redirects should just work out of the box
- Little to no "would be nice" features (re goal one)
- Easily usable yet lean as an OCI image (this is more for the project I made it for, may not matter to anyone else)

# Installation

## With Docker

You can run the Docker image and provide a bind mount like this:

```sh
docker pull ghcr.io/aosasona/chimney:latest
docker run -p 80:80 -v ./dist:/var/www/html ghcr.io/aosasona/chimney:latest
```

Although, a more practical (and recommended) usage is to use it in a multi-stage build Dockerfile, like this:

```Dockerfile
FROM node:18-alpine AS build

WORKDIR /app

COPY package.json pnpm-lock.yaml .

# Install pnpm package manager and install dependencies
RUN npm install -g pnpm

RUN pnpm install

# Copy files needed for the build, source directory and public folder
COPY astro.config.mjs tsconfig.json tailwind.config.cjs .

COPY src src

COPY public public

# Build to static HTML
RUN pnpm build

# Use chimney as final run image
FROM ghcr.io/aosasona/chimney:latest

# Copy the result of the previous build process (HTML files and the asssets; JS, CSS, Images, GIFs etc) to the default public directory
COPY --from=build /app/dist /var/www/html

# Replace the default config with our custom config
COPY chimney.toml /etc/chimney/chimney.toml

EXPOSE 80

# Start the proxy
CMD ["run"]
```

## As a standalone binary

Currently, there is no way to install via Homebrew or Cargo (this may change in the future), but you can download the binary for your platform from the [releases](https://github.com/aosasona/chimney/releases) page. If you are using Windows, there are no builds available so you could try using the next option.

## Build from source

If you are unable to or don't want to use Docker and there are no builds available for your platform, you can use Chimney by building from source:

```sh
git clone https://github.com/aosasona/chimney.git
cd chimney
cargo build --release

# and then run it
./target/release/chimney run
```

## Usage

```sh
chimney init path/to/root
chimney run -c path/to/root/chimney.toml
```

## Why not \[this other proxy/server\]?

Because I wanted to make one, and I did. That's the simple answer.

This is most definitely not what you want, and if it is, give it a go and let me know how you're using it, bugs you find and general feedback would be appreciated.

# Contributing & feedback

I would love to hear from people who are actively using this mainly for bug fixes and feature suggestions, I may or may not add your desired feature if it doesn't fit any of the goals.

> [!WARNING]
> HTTPS functionality has NOT been implemented yet, so using this standalone in production is kind of not feasible... unless you have some sort of central proxy and a bunch of containers running Chimney that you simply proxy requests to (you can probably tell what my usecase is...)

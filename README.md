> [!WARNING]
> This is still in early development, I would not recommend for production use.. yet.
>
> See the [issues](https://github.com/aosasona/chimney/issues) for things that are on the "roadmap" or missing

A lean static file server. See [this example](https://trulyao.fly.dev) deployed on Fly.io.

## Goals

- **As tiny as possible**
- **Plenty fast**
- Serve files with the correct mime-types\*
- Predictable "routing" (the way you will expect it from like Nginx or Apache eg. if `/foo` is a folder, it should resolve fine to `/foo/index.html`)
- Rewrites should be supported out of the box (needed for SPAs)
- Redirects should be as easy as possible
- Little to no "would be nice" features (re goal one)
- \*Easily usable yet lean as an OCI image (this is more for the project I made it for)

> "\*" here means "may not be 100% implemented yet, or there are other caveats"

## Installation

## With Docker

You can run the Docker image and provide a bind mount like this:

```sh
docker pull ghcr.io/aosasona/chimney:latest
docker run -p 80:80 -v ./dist:/var/www/html ghcr.io/aosasona/chimney:latest
```

Although, a more practical (and recommended) usage is to use it in a multi-stage build Dockerfile, this is a Dockerfile for an [Astro website](https://github.com/aosasona/trulyao.dev/tree/with-dockerfile):

```Dockerfile
FROM node:18-alpine AS build

WORKDIR /app

COPY package.json pnpm-lock.yaml .

RUN npm install -g pnpm

RUN pnpm install

COPY astro.config.mjs tsconfig.json tailwind.config.cjs .

COPY src src

COPY public public

RUN pnpm build

FROM ghcr.io/aosasona/chimney:latest

COPY --from=build /app/dist /var/www/html

COPY chimney.toml /etc/chimney/chimney.toml

EXPOSE 80

CMD ["run"]
```

## As a standalone binary

Currently, there is no way to install via Homebrew or Cargo (this may change in the future), but you can download the binary for your platform from the release page. If you are using Windows, there are no builds available so you could try using the next option.

## Build from source

If you are unable or don't want to use Docker and there are no builds available for your platform, you can try bulding from source:

```sh
git clone https://github.com/aosasona/chimney.git
cd chimney
cargo build --release

# and then run it
./target/release/chimney run
```

## Usage

```sh
chimney init path/to/project
chimney run -c path/to/project/chimney.toml
```

### Config reference

> [!WARNING]
> HTTPS functionality has NOT been implemented yet, so using this standalone in production is kind of not feasible... unless you have some sort of central proxy and a bunch of containers running Chimney that you simply proxy requests to (you can probably tell what my usecase is...)

| **Field**      | **Description**                                                                                                 |
| :------------- | :-------------------------------------------------------------------------------------------------------------- |
| `host`         | The IP address to bind to, default is `0.0.0.0`                                                                 |
| `port`         | The (TCP) port to run the HTTP server on, default is `80`                                                       |
| `domain_names` | The domain names that the server should respond to, this has also not been implemented yet and does nothing yet |
|                | `type`: array                                                                                                   |

## Why not \[this other proxy/server\]?

Again, the first point is the major one (and the last) for my usecase. The `caddy` image is over 1GB (really should not be), the `nginx` image is still fairly small too at around 25MB, but the reason I did not pick either for my usecase and instead opted into making this is because I:

- already had a well-defined scope, I only needed a static file server, these other proxies just also happen to be function as file servers, and they work fine but I did not want the 90% of features just to use the 10%
- wanted a bit more control, not just being able to extend as I go (also very possible with Caddy and Nginx) but things like using TOML and in turn making defining things like `redirects` and `rewrites` pretty easy and familiar (I agree too, not much of a good reason)
- wanted to use and learn Rust properly, obviously, this was a great fit for it!

> [!NOTE]
> This may not fit your usecase, have a look at [Nginx](https://www.nginx.com/) and [Caddy](https://caddyserver.com/)

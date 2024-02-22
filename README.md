> [!WARNING]
> This is still in early development, I would not recommend for production use.. yet.
>
> See the [issues](https://github.com/aosasona/chimney/issues) for things that are on the "roadmap" or missing
>
> This may not fit your usecase, have a look at [Nginx](https://www.nginx.com/) and [Caddy](https://caddyserver.com/)

A minimal static file server. See [this example](https://trulyao.fly.dev) deployed on [Fly](https://fly.io).

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

Although, a more practical (and recommended) usage is to use it in a multi-stage build Dockerfile, this is a Dockerfile for an [Astro website](https://github.com/aosasona/trulyao.dev/tree/with-dockerfile):

```Dockerfile
FROM node:18-alpine AS build

WORKDIR /app

# Install pnpm package manager and install dependencies
COPY package.json pnpm-lock.yaml .

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

### Directories

If you are using the official image, there are a few locations you might want to know about:

| **Path**                    | **Description**                                                                                                                                                                                                                                                                                                                              |
| :-------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/var/www/html`             | Similar to NGINX, this is where all your publicly available files should be, including any asset. This path was chosen since it is familiar to most people, you can change this by overriding the default config and setting `root_dir` to anywhere you want in the container                                                                |
| `/etc/chimney/chimney.toml` | This is where the default config lives, you can change this by copying your own config to wherever you desire and writing CMD as `["run", "-c", "path/to/config"]` in your custom Dockerfile                                                                                                                                                 |
| `/bin/chimney`              | This is where the Chimney binary lives in the container, since the [ENTRYPOINT](https://github.com/aosasona/chimney/blob/0aa88f573f8b7688117f978e5439db789e9c8ae1/Dockerfile#L42-L44) has been set to this path, you can easily use the "docker run" command to execute commands in the container directly without specifying `/bin/chimney` |

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
chimney init path/to/project
chimney run -c path/to/project/chimney.toml # the config filename is optional, it looks for `chimney.toml` by default in the target directory
```

### Config reference

> [!WARNING]
> HTTPS functionality has NOT been implemented yet, so using this standalone in production is kind of not feasible... unless you have some sort of central proxy and a bunch of containers running Chimney that you simply proxy requests to (you can probably tell what my usecase is...)

| **Field**               | **type**  | **Description**                                                                                                                                                                                                                                                                                                                                                                                                                                             |  **Default**   |
| :---------------------- | :-------: | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------: |
| `host`                  | `string`  | The IP address to bind to                                                                                                                                                                                                                                                                                                                                                                                                                                   |   `0.0.0.0`    |
| `port`                  | `integer` | The (TCP) port to run the HTTP server on                                                                                                                                                                                                                                                                                                                                                                                                                    |      `80`      |
| `domain_names`          |  `array`  | The domain names that the server should respond to, this has also not been implemented yet and does nothing yet                                                                                                                                                                                                                                                                                                                                             |      `[]`      |
| `enable_logging`        | `boolean` | Enable/disable request logging (what gets logged is currently limited and not quite customisable)                                                                                                                                                                                                                                                                                                                                                           |     `true`     |
| `root_dir`              | `string`  | This is where your static files and assets are located, for example `/var/www/html`. This is relative to where the config is located if it is not an absolute path, for example, if your config is located at `/Users/name/personal/chimney.toml` and the `root_dir` is set to `"public"`, this will be resolved to `/Users/name/personal/public`                                                                                                           |   `"public"`   |
| `fallback_document`     | `string`  | The file that should be served if the requested path is neither a file that exists nor a valid redirect/rewrite, leaving this blank is also allowed and will just send the status code with no body. A good usecase would be setting this to `index.html` if you are serving an SPA, or `404.html` if you have an Astro site for example                                                                                                                    | `"index.html"` |
| `https.enable`          | `boolean` | `unimplemented`                                                                                                                                                                                                                                                                                                                                                                                                                                             |    `false`     |
| `https.auto_redirect`   | `boolean` | `unimplemented`                                                                                                                                                                                                                                                                                                                                                                                                                                             |    `false`     |
| `https.port`            | `integer` | `unimplemented`                                                                                                                                                                                                                                                                                                                                                                                                                                             |     `443`      |
| `https.use_self_signed` | `boolean` | `unimplemented`                                                                                                                                                                                                                                                                                                                                                                                                                                             |    `false`     |
| `https.cert_file`       | `string`  | `unimplemented`                                                                                                                                                                                                                                                                                                                                                                                                                                             |    _`nil`_     |
| `https.key_file`        | `string`  | `unimplemented`                                                                                                                                                                                                                                                                                                                                                                                                                                             |    _`nil`_     |
| `rewrites`              |   `map`   | A rewrite generally maintains the same URL but serves something different, the file doesn't even need to exist. A rewrite could in fact point to a redirect, the leading slash is required when defining a rewrite. For example, if you have a rewrite defined as `"/foo" = "page.html"`, even though `foo` is not a real file, when the server receives a request for `/foo`, it will read and serve the `page.html` file instead without the user knowing |                |
| `headers`               |   `map`   | Extra headers you want to append to every response the server sends out                                                                                                                                                                                                                                                                                                                                                                                     |    _`nil`_     |
| `redirects`             |   `map`   | A redirect maps a path to en external (or even internal) URL, unlike rewrites, a redirect does not read or serve a file, it simply takes the user away to the specified URL, and replays the request (useful for POSTs) if configured to. For example, `"/foo" =  { to = "https://example.com", replay = false }` will take the user to `example.com` anytime they visit `yourwebsite.com/foo` but will NOT replay the request if it was a POST or similar. |    _`nil`_     |

You can find sample config files [here](https://github.com/aosasona/chimney/blob/master/examples/basic/chimney.toml) and [here](https://github.com/aosasona/chimney/blob/master/examples/trulyao/chimney.toml).

## Why not \[this other proxy/server\]?

Again, the first point is the major one (and the last) for my usecase. The `caddy` image is over 1GB (really should not be), the `nginx` image is still fairly small too at around 25MB, but the reason I did not pick either for my usecase and instead opted into making this is because I:

- already had a well-defined scope, I only needed a static file server, these other proxies just also happen to be function as file servers, and they work fine but I did not want the 90% of features just to use the 10%
- wanted a bit more control, not just being able to extend as I go (also very possible with Caddy and Nginx) but things like using TOML and in turn making defining things like `redirects` and `rewrites` pretty easy and familiar (I agree too, not much of a good reason)
- wanted to use and learn Rust properly, obviously, this was a great fit for it!

# Contributing & feedback

I would love to hear from people who are actively using this mainly for bug fixes and feature suggestions, I may or may not add your desired feature if it doesn't fit any of the goals.

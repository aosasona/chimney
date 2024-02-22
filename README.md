# Chimney

> [!WARNING]
> This is still in early development, I would not recommend for production use.. yet

A lean static file server. See [this example](https://trulyao.fly.dev) deployed on Fly.io.

# Goals

- **As tiny as possible**
- **Plenty fast**
- Serve files with the correct mime-types\*
- Predictable "routing" (the way you will expect it from like Nginx or Apache eg. if `/foo` is a folder, it should resolve fine to `/foo/index.html`)
- Rewrites should be supported out of the box (needed for SPAs)
- Redirects should be as easy as possible
- Little to no "would be nice" features (re goal one)
- \*Easily usable yet lean as an OCI image (this is more for the project I made it for)

> "\*" here means may not be 100% implemented yet, or there are other caveats

# Why not \[this other proxy/server\]?

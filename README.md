# rust-hyper-server

## What is this?
Static file server and REST API in rust using [hyper v1.0.0-rc.4](https://github.com/hyperium/hyper) and [tokio](https://tokio.rs/)

Features:
* [toml configuration files](https://github.com/aaronriekenberg/rust-hyper-server/tree/main/config)
* any number HTTP 1.x or HTTP 2 servers using hyper, each listening on 1 configured TCP or UNIX socket
  * includes a [`pin_project` enum wrapping H1 and H2 hyper connections](https://github.com/aaronriekenberg/rust-hyper-server/blob/main/src/server/h1h2conn.rs) for polling and graceful shutdown
* structured logging with spans for incoming connections and requests
* static file server using [hyper-staticfile](https://github.com/stephank/hyper-staticfile)
  * precompressed static files (bz and/or gz)
* configurable rules list using regular expressions for cache control response headers on static files
* server connection tracking
  * timeouts with graceful shutdown
  * track connection age, requests per connection, configurable connection limit
  * historical connection metrics
* generic `handlers::RequestHandler` async trait to build REST-style endpoints
  * asynchronously run configured shell commands and return response as json
  * connection info
  * request info
  * version info

## Github Actions
When the release build is too slow on your Raspberry Pi: Use [github actions](https://github.com/aaronriekenberg/rust-hyper-server/actions) to cross-compile.

## Memory Usage
This app runs in about 8 megabytes of resident memory (RSS) on a 64-bit Raspberry Pi.

```
$ ps -eo pid,pmem,rss,vsz,comm --sort -rss

    PID %MEM   RSS    VSZ COMMAND         
  10791  0.8  7744 890832 rust-hyper-serv
```

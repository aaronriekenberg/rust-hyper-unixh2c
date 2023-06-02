# rust-hyper-server

## What is this?
Similar to [rust-fastcgi](https://github.com/aaronriekenberg/rust-fastcgi), but:

* Using [hyper](https://hyper.rs/) to do cleartext HTTP/2 (h2c) or HTTP/1.1 over UNIX and/or TCP sockets.
* Using [hyper-staticfile](https://github.com/stephank/hyper-staticfile) to serve static files.

## Github Actions
When the release build is too slow on your Raspberry Pi: Use [github actions](https://github.com/aaronriekenberg/rust-hyper-server/actions) to cross-compile.

## Memory Usage
This app runs in about 3.5 megabytes of resident memory (RSS) on a 64-bit Raspberry Pi.

```
$ ps -eo pid,pmem,rss,vsz,comm --sort -rss

    PID %MEM   RSS    VSZ COMMAND         
  16285  0.3  3324 478072 rust-hyper-unix
```

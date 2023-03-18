# rust-hyper-unixh2c

## What is this?
Similar to [rust-fastcgi](https://github.com/aaronriekenberg/rust-fastcgi), but:

* Using [hyper](https://hyper.rs/) to do cleartext HTTP2 (h2c) over a unix socket
* Running this with Caddy reverse_proxy as:

```
       handle /cgi-bin/* {
                reverse_proxy unix+h2c//path/to/unix/socket
       }
```

## Github Actions
When the release build is too slow on your Raspberry Pi: Use [github actions](https://github.com/aaronriekenberg/rust-hyper-unixh2c/actions) to cross-compile.

## Memory Usage
This app runs in about 3.5 megabytes of resident memory (RSS) on a 64-bit Raspberry Pi.

```
$ ps -eo pid,pmem,rss,vsz,comm --sort -rss

    PID %MEM   RSS    VSZ COMMAND         
  16285  0.3  3324 478072 rust-hyper-unix
```

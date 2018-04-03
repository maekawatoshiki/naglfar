# Naglfar

[![CircleCI](https://img.shields.io/circleci/project/github/maekawatoshiki/naglfar/master.svg?style=flat-square)](https://circleci.com/gh/maekawatoshiki/naglfar)
[![codecov](https://img.shields.io/codecov/c/github/maekawatoshiki/naglfar/master.svg?style=flat-square)](https://codecov.io/gh/maekawatoshiki/naglfar)
[![](http://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](./LICENSE)

Naglfar is a toy web browser in Rust.

![Naglfar](https://raw.githubusercontent.com/maekawatoshiki/naglfar/master/screenshot.gif)

# Run

## Requisites

- Latest Rust (recommend [rustup](https://www.rustup.rs/))
- GTK (for gtk-rs)

## Build & Run

Try with the local html file(e.g. ./example/test.html):

```sh
$ cargo run file:///../../example/test.html
```

Try with the html file on the Internet:
(But most of web sites are too much for Naglfar...)

```sh
$ cargo run http(s)://example.com/index.html
```

# Reference

Great thanks to [robinson](https://github.com/mbrubeck/robinson)

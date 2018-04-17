# Naglfar

[![CircleCI](https://circleci.com/gh/maekawatoshiki/naglfar.svg?style=shield)](https://circleci.com/gh/maekawatoshiki/naglfar)
[![codecov](https://codecov.io/gh/maekawatoshiki/naglfar/branch/master/graph/badge.svg)](https://codecov.io/gh/maekawatoshiki/naglfar)
[![](http://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Naglfar is a toy web browser in Rust.

![Naglfar](https://raw.githubusercontent.com/maekawatoshiki/naglfar/master/screenshot.gif)

# Running

## Requisites

- Latest Rust (recommend [rustup](https://www.rustup.rs/))
- GTK (for gtk-rs)

## Building & Running

Try with the local html file(e.g. ./example/test.html):

```sh
$ cargo run file:///../../example/test.html
```

Try with the html file on the Internet:
(But most of web sites are too much for Naglfar...)

```sh
$ cargo run https://maekawatoshiki.github.io/naglfar/example/test.html
```

# Reference

Great thanks to [robinson](https://github.com/mbrubeck/robinson)

# Naglfar

[![CircleCI](https://circleci.com/gh/maekawatoshiki/naglfar.svg?style=shield)](https://circleci.com/gh/maekawatoshiki/naglfar)
[![codecov](https://codecov.io/gh/maekawatoshiki/naglfar/branch/master/graph/badge.svg)](https://codecov.io/gh/maekawatoshiki/naglfar)
[![](http://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Naglfar is a toy web browser in Rust.

**I'm focusing on developing a toy JavaScript engine: ![Rapidus](https://github.com/maekawatoshiki/rapidus).**

![Naglfar](https://raw.githubusercontent.com/maekawatoshiki/naglfar/master/screenshot.gif)

# Try Naglfar

## Requisites

- Latest Rust (recommend [rustup](https://www.rustup.rs/))
- GTK (for gtk-rs)

## Run

A blank window will appear if you run Naglfar with no option.

```sh
$ cargo run
```

Give the local html file (e.g. ./example/test.html):

```sh
$ cargo run file://`pwd`/example/test.html
```

Give the html file on the Internet:
(But most of web sites are too much for Naglfar...)

```sh
$ cargo run https://maekawatoshiki.github.io/naglfar/example/test.html
```

# Reference

Great thanks to [robinson](https://github.com/mbrubeck/robinson)

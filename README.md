# tracing-line-filter

A [`tracing`] filter for enabling individual [spans] and [events] by line
number.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
![maintenance status][maint-badge]

[crates-badge]: https://img.shields.io/crates/v/tracing-line-filter.svg
[crates-url]: https://crates.io/crates/tracing-subscriber
[docs-badge]: https://docs.rs/tracing-line-filter/badge.svg
[docs-url]: https://docs.rs/tracing-line-filter/0.1
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/hawkw/line-filter/workflows/CI/badge.svg
[actions-url]:https://github.com//hawkw/line-filter/actions?query=workflow%3ACI
[maint-badge]: https://img.shields.io/badge/maintenance-experimental-blue.svg

[`tracing`] is a framework for instrumenting Rust programs to collect
scoped, structured, and async-aware diagnostics. The [`tracing-subscriber`]
crate's [`EnvFilter`] type provides a mechanism for controlling what
`tracing` [spans] and [events] are collected by matching their targets,
verbosity levels, and fields. In some cases, though, it can be useful to
toggle on or off individual spans or events with a higher level of
granularity. Therefore, this crate provides a filtering [`Layer`] that
enables individual spans and events based on their module path/file path and
line numbers.

Since the implementation of this filter is rather simple, the source code of
this crate is also useful as an example to `tracing` users who want to
implement their own filtering logic.

# Usage

First, add this to your Cargo.toml:

```toml
tracing-line-filter = "0.1"
```

## Examples

Enabling events by line: 

```rust
use tracing_line_filter::LineFilter;
mod some_module {
    pub fn do_stuff() {
        tracing::info!("i'm doing stuff");
        tracing::debug!("i'm also doing stuff!");
    }
}

fn main() {
    use tracing_subscriber::prelude::*;

    let mut filter = LineFilter::default();
    filter
        .enable_by_mod("my_crate::some_module", 6)
        .enable_by_mod("my_crate", 25)
        .enable_by_mod("my_crate", 27);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(filter)
        .init();

    tracing::info!("i'm not enabled");
    tracing::debug!("i'm enabled!");
    some_module::do_stuff();
    tracing::trace!("hi!");
}
```

Chaining a [`LineFilter`] with a `tracing_subscriber` [`EnvFilter`]:

```rust
use tracing_line_filter::LineFilter;
use tracing_subscriber::EnvFilter;

mod some_module {
    pub fn do_stuff() {
        tracing::info!("i'm doing stuff");
        tracing::debug!("i'm also doing stuff!");
        // This won't be enabled, because it's at the TRACE level, and the
        // `EnvFilter` only enables up to the DEBUG level.
        tracing::trace!("doing very verbose stuff");
    }
}

fn main() {
    use tracing_subscriber::prelude::*;

    let mut filter = LineFilter::default();
    filter
        .enable_by_mod("with_env_filter", 30)
        .enable_by_mod("with_env_filter", 33)
        // use an `EnvFilter` that enables DEBUG and lower in `some_module`,
        // and everything at the ERROR level.
        .with_env_filter(EnvFilter::new("error,with_env_filter::some_module=debug"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(filter)
        .init();

    tracing::info!("i'm not enabled");
    tracing::debug!("i'm enabled!!");
    some_module::do_stuff();
    tracing::trace!("hi!");

    // This will be enabled by the `EnvFilter`.
    tracing::error!("an error!");
}
```

[`tracing`]: https://docs.rs/tracing
[spans]: https://docs.rs/tracing/latest/tracing/#spans
[events]: https://docs.rs/tracing/latest/tracing/#events
[`tracing-subscriber`]: https://docs.rs/tracing-subscriber
[`EnvFilter`]: https://docs.rs/tracing-subscriber/latest/struct.EnvFilter.html
[`LineFilter`]: https://docs.rs/tracing-line-filter/latest/struct.LineFilter.html
[`Layer`]: https://docs.rs/tracing-subscriber/latest/trait.Layer.html
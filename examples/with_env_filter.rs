use line_filter::LineFilter;
use tracing_subscriber::EnvFilter;

mod some_module {
    pub fn do_stuff() {
        tracing::info!("i'm doing stuff");
        tracing::debug!("i'm also doing stuff!");
        tracing::trace!("doing very verbose stuff");
    }
}

fn main() {
    use tracing_subscriber::prelude::*;

    let filter = LineFilter::new(vec![
        ("with_env_filter".to_owned(), 28),
        ("with_env_filter".to_owned(), 30)
    ])
        // use an ``EnvFilter` that enables DEBUG and lower in `some_module`.
        .with_env_filter(EnvFilter::new("with_env_filter::some_module=debug"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(filter)
        .init();

    tracing::info!("i'm not enabled");
    tracing::debug!("i'm enabled!!");
    some_module::do_stuff();
    tracing::trace!("hi!");
}
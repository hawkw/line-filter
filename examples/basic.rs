use line_filter::LineFilter;

mod some_module {
    pub fn do_stuff() {
        tracing::info!("i'm doing stuff");
        tracing::debug!("i'm also doing stuff!");
    }
}

fn main() {
    use tracing_subscriber::prelude::*;

    let filter = LineFilter::new(vec![
        ("basic::some_module".to_owned(), 6),
        ("basic".to_owned(), 25),
        ("basic".to_owned(), 27)
    ]);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(filter)
        .init();

    tracing::info!("i'm not enabled");
    tracing::debug!("i'm enabled!");
    some_module::do_stuff();
    tracing::trace!("hi!");
}
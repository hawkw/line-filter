use std::collections::HashSet;
use std::borrow::Cow;
use tracing_core::{subscriber::Interest, Metadata, Subscriber};
use tracing_subscriber::{filter::EnvFilter, layer::{self, Layer}};

#[derive(Debug)]
pub struct LineFilter {
    enabled: HashSet<(Cow<'static, str>, u32)>,
    env: Option<EnvFilter>,
}

impl LineFilter {
    pub fn new(enabled: impl IntoIterator<Item = (String, u32)>) -> Self {
        Self {
            enabled: enabled.into_iter().map(|(module, line)|(Cow::Owned(module), line)).collect(),
            env: None,
        }
    }

    /// Composes `self` with an `EnvFilter` that will be checked for spans and
    /// events if they are not in the list of enabled `(target, line)` pairs.
    pub fn with_env_filter(self, env: EnvFilter) -> Self {
        Self {
            env: Some(env),
            ..self
        }
    }
}

impl<S: Subscriber> Layer<S> for LineFilter
where
    EnvFilter: Layer<S>,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if let Some(line) = metadata.line() {
            let location = (Cow::Borrowed(metadata.target()), line);

            if self.enabled.contains(&location) {
                return Interest::always();
            }
        }

        self.env
            .as_ref()
            .map(|env| env.register_callsite(metadata))
            .unwrap_or_else(Interest::never)
    }


    fn enabled(&self, metadata: &Metadata<'_>, cx: layer::Context<'_, S>) -> bool {
        if let Some(line) = metadata.line() {
            let location = (Cow::Borrowed(metadata.target()), line);

            if self.enabled.contains(&location) {
                return true;
            }
        }


        self.env
            .as_ref()
            .map(|env| env.enabled(metadata, cx))
            .unwrap_or(false)
    }
}

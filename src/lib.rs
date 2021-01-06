use std::borrow::Cow;
use std::collections::HashSet;
use tracing_core::{subscriber::Interest, Metadata, Subscriber};
use tracing_subscriber::{
    filter::EnvFilter,
    layer::{self, Layer},
};

#[derive(Debug, Default)]
pub struct LineFilter {
    by_module: HashSet<(Cow<'static, str>, u32)>,
    by_file: HashSet<(Cow<'static, str>, u32)>,
    env: Option<EnvFilter>,
}

impl LineFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Composes `self` with an `EnvFilter` that will be checked for spans and
    /// events if they are not in the list of by_module `(target, line)` pairs.
    pub fn with_env_filter(&mut self, env: EnvFilter) -> &mut Self {
        self.env = Some(env);
        self
    }

    pub fn with_modules<I>(&mut self, modules: impl IntoIterator<Item = (I, u32)>) -> &mut Self
    where
        I: Into<Cow<'static, str>>,
    {
        let modules = modules
            .into_iter()
            .map(|(module, line)| (module.into(), line));
        self.by_module.extend(modules);
        self
    }

    pub fn with_files<I>(&mut self, files: impl IntoIterator<Item = (I, u32)>) -> &mut Self
    where
        I: Into<Cow<'static, str>>,
    {
        let modules = files.into_iter().map(|(file, line)| (file.into(), line));
        self.by_file.extend(modules);
        self
    }

    pub fn enable_by_mod(&mut self, module: impl Into<Cow<'static, str>>, line: u32) -> &mut Self {
        self.by_module.insert((module.into(), line));
        self
    }

    pub fn enable_by_file(&mut self, file: impl Into<Cow<'static, str>>, line: u32) -> &mut Self {
        self.by_file.insert((file.into(), line));
        self
    }

    fn contains(&self, metadata: &Metadata<'_>) -> bool {
        if let Some(line) = metadata.line() {
            if let Some(module) = metadata.module_path() {
                let location = (Cow::Borrowed(module), line);
                if self.by_module.contains(&location) {
                    return true;
                }
            }

            if let Some(file) = metadata.file() {
                let location = (Cow::Borrowed(file), line);
                if self.by_file.contains(&location) {
                    return true;
                }
            }
        }

        false
    }
}

impl<S: Subscriber> Layer<S> for LineFilter
where
    EnvFilter: Layer<S>,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.contains(metadata) {
            return Interest::always();
        }

        self.env
            .as_ref()
            .map(|env| env.register_callsite(metadata))
            .unwrap_or_else(Interest::never)
    }

    fn enabled(&self, metadata: &Metadata<'_>, cx: layer::Context<'_, S>) -> bool {
        if self.contains(metadata) {
            return true;
        }

        self.env
            .as_ref()
            .map(|env| env.enabled(metadata, cx))
            .unwrap_or(false)
    }
}

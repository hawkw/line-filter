//! A [`tracing`] filter for enabling individual [spans] and [events] by line
//! number.
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. The [`tracing-subscriber`]
//! crate's [`EnvFilter`] type provides a mechanism for controlling what
//! `tracing` [spans] and [events] are collected by matching their targets,
//! verbosity levels, and fields. In some cases, though, it can be useful to
//! toggle on or off individual spans or events with a higher level of
//! granularity. Therefore, this crate provides a filtering [`Layer`] that
//! enables individual spans and events based on their module path/file path and
//! line numbers.
//!
//! Since the implementation of this filter is rather simple, the source code of
//! this crate is also useful as an example to `tracing` users who want to
//! implement their own filtering logic.
//!
//! # Usage
//!
//! First, add this to your Cargo.toml:
//!
//! ```toml
//! tracing-line-filter = "0.1"
//! ```
//!
//! ## Examples
//!
//! Enabling events by line:
//!
//! ```rust
//! use tracing_line_filter::LineFilter;
//! mod some_module {
//!     pub fn do_stuff() {
//!         tracing::info!("i'm doing stuff");
//!         tracing::debug!("i'm also doing stuff!");
//!     }
//! }
//!
//! fn main() {
//!     use tracing_subscriber::prelude::*;
//!
//!     let mut filter = LineFilter::default();
//!     filter
//!         .enable_by_mod("my_crate::some_module", 6)
//!         .enable_by_mod("my_crate", 25)
//!         .enable_by_mod("my_crate", 27);
//!
//!     tracing_subscriber::registry()
//!         .with(tracing_subscriber::fmt::layer().pretty())
//!         .with(filter)
//!         .init();
//!
//!     tracing::info!("i'm not enabled");
//!     tracing::debug!("i'm enabled!");
//!     some_module::do_stuff();
//!     tracing::trace!("hi!");
//! }
//! ```
//!
//! Chaining a [`LineFilter`] with a `tracing_subscriber` [`EnvFilter`]:
//!
//! ```rust
//! use tracing_line_filter::LineFilter;
//! use tracing_subscriber::EnvFilter;
//!
//! mod some_module {
//!     pub fn do_stuff() {
//!         tracing::info!("i'm doing stuff");
//!         tracing::debug!("i'm also doing stuff!");
//!         // This won't be enabled, because it's at the TRACE level, and the
//!         // `EnvFilter` only enables up to the DEBUG level.
//!         tracing::trace!("doing very verbose stuff");
//!     }
//! }
//!
//! fn main() {
//!     use tracing_subscriber::prelude::*;
//!
//!     let mut filter = LineFilter::default();
//!     filter
//!         .enable_by_mod("with_env_filter", 30)
//!         .enable_by_mod("with_env_filter", 33)
//!         // use an `EnvFilter` that enables DEBUG and lower in `some_module`,
//!         // and everything at the ERROR level.
//!         .with_env_filter(EnvFilter::new("error,with_env_filter::some_module=debug"));
//!
//!     tracing_subscriber::registry()
//!         .with(tracing_subscriber::fmt::layer().pretty())
//!         .with(filter)
//!         .init();
//!
//!     tracing::info!("i'm not enabled");
//!     tracing::debug!("i'm enabled!!");
//!     some_module::do_stuff();
//!     tracing::trace!("hi!");
//!
//!     // This will be enabled by the `EnvFilter`.
//!     tracing::error!("an error!");
//! }
//! ```
//!
//! [`tracing`]: https://docs.rs/tracing
//! [spans]: https://docs.rs/tracing/latest/tracing/#spans
//! [events]: https://docs.rs/tracing/latest/tracing/#events
//! [`tracing-subscriber`]: https://docs.rs/tracing-subscriber
//! [`EnvFilter`]: tracing_subscriber::EnvFilter
//! [`Layer`]: tracing_subscriber::Layer

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use tracing_core::{subscriber::Interest, Metadata, Subscriber};
use tracing_subscriber::{
    filter::EnvFilter,
    layer::{self, Layer},
};

/// A filter for enabling spans and events by file/module path and line number.
#[derive(Debug, Default)]
pub struct LineFilter {
    by_module: HashSet<(Cow<'static, str>, u32)>,
    by_file: HashSet<(Cow<'static, str>, u32)>,
    env: Option<EnvFilter>,
}

/// Indicates a file path was invalid for use in a `LineFilter`.
#[derive(Debug)]
pub struct BadPath {
    path: PathBuf,
    message: &'static str,
}

impl LineFilter {
    /// Returns a new `LineFilter`.
    ///
    /// By default, no spans and events are enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Composes `self` with an [`EnvFilter`] that will be checked for spans and
    /// events if they are not in the lists of enabled `(module, line)` and
    /// `(file, line)` pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_subscriber::EnvFilter;
    /// use tracing_line_filter::LineFilter;
    ///
    /// let mut filter = LineFilter::default();
    /// filter
    ///     .enable_by_mod("my_crate", 28)
    ///     .enable_by_mod("my_crate::my_module", 16)
    ///     // use an ``EnvFilter` that enables DEBUG and lower in `some_module`, and
    ///     // all ERROR spans or events, regardless of location.
    ///     .with_env_filter(EnvFilter::new("error,my_crate::some_other_module=debug"));
    /// ```
    pub fn with_env_filter(&mut self, env: EnvFilter) -> &mut Self {
        self.env = Some(env);
        self
    }

    /// Enable a span or event in the Rust module `module` on line `line`.
    ///
    /// # Notes
    ///
    /// * Module paths should include the name of the crate. For
    ///   example, the module `my_module` in `my_crate` would have path
    ///  `my_crate::my_module`.
    /// * If no span or event exists at the specified location, or if the module
    ///   path does not exist, this will silently do nothing.
    /// * Line numbers are relative to the start of the *file*, not to the start
    ///   of the module. If a module does not have its own file (i.e., it's
    ///   defined like `mod my_module { ... }`), the line number is relative to
    ///   the containing file.
    ///
    /// # Examples
    ///
    /// Enabling an event:
    /// ```
    /// use tracing_line_filter::LineFilter;
    ///
    /// mod my_module {
    ///     pub fn do_stuff() {
    ///         tracing::info!("doing stuff!")
    ///         // ...
    ///     }
    /// }
    ///
    /// // Build a line filter to enable the event in `do_stuff`.
    /// let mut filter = LineFilter::default();
    /// filter.enable_by_module("my_crate::my_module", 5);
    ///
    /// // Build a subscriber and enable that filter.
    /// use tracing_subscriber::prelude::*;
    ///
    /// tracing_subscriber::registry()
    ///     .with(tracing_subscriber::fmt::layer())
    ///     .with(filter)
    ///     .init();
    ///
    /// // Now, the event is enabled!
    /// do_stuff();
    /// ```
    ///
    /// The [`std::module_path!()`] macro can be used to enable an event in the
    /// current module:
    /// ```
    /// use tracing_line_filter::LineFilter;
    ///
    /// pub fn do_stuff() {
    ///     tracing::info!("doing stuff!")
    ///     // ...
    /// }
    ///
    /// let mut filter = LineFilter::default();
    /// filter.enable_by_module(module_path!(), 4);
    ///
    ///  // ...
    /// ```
    pub fn enable_by_mod(&mut self, module: impl Into<Cow<'static, str>>, line: u32) -> &mut Self {
        self.by_module.insert((module.into(), line));
        self
    }

    /// Enable a span or event in the file `file` on line `line`.
    ///
    /// # Notes
    ///
    /// These file paths must match the file paths emitted by the
    /// [`std::file!()`] macro. In particular:
    ///
    /// * Paths must be absolute.
    /// * Paths must be Rust source code files.
    /// * Paths must be valid UTF-8.
    ///
    /// This method validates paths and returns an error if the path is not
    /// valid for use in a `LineFilter`.
    ///
    /// Since these paths are absolute, files in Cargo dependencies will include
    /// their full path in the local Cargo registry. For example:
    /// ```text
    /// /home/eliza/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.0.0/src/util/trace.rs
    /// ```
    ///
    /// Therefore, it can be challenging for humans to determine the correct
    /// path to a file, especially when it is in a dependency. For this reason,
    /// it's likely best to prefer Rust module paths rather than file paths when
    /// accepting input from users directly. Enabling events and spans by file
    /// paths is primarily intended for use by automated tools.
    pub fn enable_by_file(
        &mut self,
        file: impl AsRef<Path>,
        line: u32,
    ) -> Result<&mut Self, BadPath> {
        let file = file.as_ref();
        if !file.is_absolute() {
            return Err(BadPath::new(file, "file paths must be absolute"));
        }

        if file.extension().and_then(std::ffi::OsStr::to_str) != Some("rs") {
            return Err(BadPath::new(file, "files must be Rust source code files"));
        }

        let file = file
            .to_str()
            .ok_or_else(|| BadPath::new(file, "file paths must be valid utf-8"))?
            .to_owned();

        self.by_file.insert((Cow::Owned(file), line));
        Ok(self)
    }

    /// Enable a set of spans or events by module path.
    ///
    /// This is equivalent to repeatedly calling [`enable_by_mod`].
    ///
    ///
    /// # Examples
    /// ```
    /// use tracing_line_filter::LineFilter;
    ///
    /// mod foo {
    ///     pub fn do_stuff() {
    ///         tracing::info!("doing stuff!")
    ///         // ...
    ///     }
    /// }
    ///
    /// mod bar {
    ///     pub fn do_other_stuff() {
    ///         tracing::debug!("doing some different stuff...")
    ///         // ...
    ///     }
    /// }
    ///
    /// // Build a line filter to enable the events in `do_stuff`
    /// // and `do_other_stuff`.
    /// let mut filter = LineFilter::default();
    /// filter.with_modules(vec![
    ///    ("my_crate::foo", 5),
    ///    ("my_crate::bar", 12)
    /// ]);
    ///
    /// // Build a subscriber and enable that filter.
    /// use tracing_subscriber::prelude::*;
    ///
    /// tracing_subscriber::registry()
    ///     .with(tracing_subscriber::fmt::layer())
    ///     .with(filter)
    ///     .init();
    ///
    /// // Now, the events are enabled!
    /// do_stuff();
    /// do_other_stuff();
    /// ```
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

    /// Enable a set of spans or events by file path.
    ///
    /// This is equivalent to repeatedly calling [`enable_by_file`], and follows
    /// the same path validation rules as that method. See the documentation for
    /// [`enable_by_file`] for details.
    pub fn with_files<I>(
        &mut self,
        files: impl IntoIterator<Item = (I, u32)>,
    ) -> Result<&mut Self, BadPath>
    where
        I: AsRef<Path>,
    {
        for (file, line) in files {
            self.enable_by_file(file, line)?;
        }
        Ok(self)
    }

    fn contains(&self, metadata: &Metadata<'_>) -> bool {
        if let Some(line) = metadata.line() {
            let module = metadata.module_path().unwrap_or_else(|| metadata.target());
            let location = (Cow::Borrowed(module), line);
            if self.by_module.contains(&location) {
                return true;
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

// === impl BadPath ===

impl fmt::Display for BadPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid path '{}': {}",
            self.path.display(),
            self.message
        )
    }
}

impl std::error::Error for BadPath {}

impl BadPath {
    fn new(path: &Path, message: &'static str) -> Self {
        Self {
            path: path.to_path_buf(),
            message,
        }
    }
}

//! `log` compatible logger to the `HiLog` logging system on OpenHarmony
//!
//! This crate is in its very early stages and still under development.
//! It's partially based on [`env_logger`], in particular the filtering
//! is compatible with [`env_logger`].
//!
//! [`env_logger`]: https://docs.rs/env_logger/latest/env_logger/
mod ohfmt;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::io;
use hilog_sys::{LogLevel, LogType, OH_LOG_IsLoggable, OH_LOG_Print};
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use crate::ohfmt::{Buffer, HilogFormatter, TimestampPrecision};

/// Service domain of logs
///
/// The user can set this value as required. The value can be used
/// when filtering `hilog` logs.
#[derive(Copy, Clone, Default, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct LogDomain(u16);

impl LogDomain {

    /// Creates a new LogDomain
    ///
    /// Valid values are 0-0xFFFF.
    pub fn new(domain: u16) -> Self {
        Self(domain)
    }
}


fn hilog_log(log_type: LogType, level: LogLevel, domain: LogDomain, tag: &CStr, msg: &CStr) {
    let _res = unsafe {
        OH_LOG_Print(
            log_type,
            level,
            domain.0.into(),
            tag.as_ptr(),
            c"%{public}s".as_ptr(),
            msg.as_ptr()
        )
    };
}

#[derive(Default)]
pub struct Builder {
    filter: env_filter::Builder,
    log_domain: LogDomain,
    format: ohfmt::builder::Builder,
    writer: ohfmt::writer::Builder,
    built: bool,
}

impl Builder {
    pub fn new() -> Builder {
        Default::default()
    }


    /// Sets the Service domain for the logs
    ///
    /// Users can set a custom domain, which allows filtering by hilogd.
    pub fn set_domain(&mut self, domain: LogDomain) -> &mut Self {
        self.log_domain = domain;
        self
    }

    /// Adds a directive to the filter for a specific module.
    ///
    /// # Examples
    ///
    /// Only include messages for info and above for logs in `path::to::module`:
    ///
    /// ```
    /// use env_filter::Builder;
    /// use log::LevelFilter;
    ///
    /// let mut builder = Builder::new();
    ///
    /// builder.filter_module("path::to::module", LevelFilter::Info);
    /// ```
    pub fn filter_module(&mut self, module: &str, level: LevelFilter) -> &mut Self {
        self.filter.filter_module(module, level);
        self
    }

    /// Adds a directive to the filter for all modules.
    ///
    /// # Examples
    ///
    /// Only include messages for info and above for logs globally:
    ///
    /// ```
    /// use env_filter::Builder;
    /// use log::LevelFilter;
    ///
    /// let mut builder = Builder::new();
    ///
    /// builder.filter_level(LevelFilter::Info);
    /// ```
    pub fn filter_level(&mut self, level: LevelFilter) -> &mut Self {
        self.filter.filter_level(level);
        self
    }

    /// Adds filters to the logger.
    ///
    /// The given module (if any) will log at most the specified level provided.
    /// If no module is provided then the filter will apply to all log messages.
    ///
    /// # Examples
    ///
    /// Only include messages for info and above for logs in `path::to::module`:
    ///
    /// ```
    /// use env_filter::Builder;
    /// use log::LevelFilter;
    ///
    /// let mut builder = Builder::new();
    ///
    /// builder.filter(Some("path::to::module"), LevelFilter::Info);
    /// ```
    pub fn filter(&mut self, module: Option<&str>, level: LevelFilter) -> &mut Self {
        self.filter.filter(module, level);
        self
    }

    /// Sets the format function for formatting the log output.
    ///
    /// This function is called on each record logged and should format the
    /// log record and output it to the given [`HilogFormatter`].
    ///
    /// The format function is expected to output the string directly to the
    /// `HilogFormatter` so that implementations can use the [`std::fmt`] macros
    /// to format and output without intermediate heap allocations. The default
    /// `env_logger` HilogFormatter takes advantage of this.
    ///
    /// When the `color` feature is enabled, styling via ANSI escape codes is supported and the
    /// output will automatically respect [`Builder::write_style`].
    ///
    /// # Examples
    ///
    /// Use a custom format to write only the log message:
    ///
    /// ```
    /// use std::io::Write;
    /// use env_logger::Builder;
    ///
    /// let mut builder = Builder::new();
    ///
    /// builder.format(|buf, record| writeln!(buf, "{}", record.args()));
    /// ```
    ///
    /// [`HilogFormatter`]: fmt/struct.HilogFormatter.html
    /// [`String`]: https://doc.rust-lang.org/stable/std/string/struct.String.html
    /// [`std::fmt`]: https://doc.rust-lang.org/std/fmt/index.html
    pub fn format<F>(&mut self, format: F) -> &mut Self
    where
        F: Fn(&mut HilogFormatter, &Record<'_>) -> io::Result<()> + Sync + Send + 'static,
    {
        self.format.custom_format = Some(Box::new(format));
        self
    }

    /// Use the default format.
    ///
    /// This method will clear any custom format set on the builder.
    pub fn default_format(&mut self) -> &mut Self {
        self.format = Default::default();
        self
    }

    /// Whether or not to write the level in the default format.
    pub fn format_level(&mut self, write: bool) -> &mut Self {
        self.format.format_level = write;
        self
    }

    /// Whether or not to write the module path in the default format.
    pub fn format_module_path(&mut self, write: bool) -> &mut Self {
        self.format.format_module_path = write;
        self
    }

    /// Whether or not to write the target in the default format.
    pub fn format_target(&mut self, write: bool) -> &mut Self {
        self.format.format_target = write;
        self
    }

    /// Configures the amount of spaces to use to indent multiline log records.
    /// A value of `None` disables any kind of indentation.
    pub fn format_indent(&mut self, indent: Option<usize>) -> &mut Self {
        self.format.format_indent = indent;
        self
    }

    /// Configures if timestamp should be included and in what precision.
    pub fn format_timestamp(&mut self, timestamp: Option<TimestampPrecision>) -> &mut Self {
        self.format.format_timestamp = timestamp;
        self
    }

    /// Configures the timestamp to use second precision.
    pub fn format_timestamp_secs(&mut self) -> &mut Self {
        self.format_timestamp(Some(TimestampPrecision::Seconds))
    }

    /// Configures the timestamp to use millisecond precision.
    pub fn format_timestamp_millis(&mut self) -> &mut Self {
        self.format_timestamp(Some(TimestampPrecision::Millis))
    }

    /// Configures the timestamp to use microsecond precision.
    pub fn format_timestamp_micros(&mut self) -> &mut Self {
        self.format_timestamp(Some(TimestampPrecision::Micros))
    }

    /// Configures the timestamp to use nanosecond precision.
    pub fn format_timestamp_nanos(&mut self) -> &mut Self {
        self.format_timestamp(Some(TimestampPrecision::Nanos))
    }

    /// Configures the end of line suffix.
    pub fn format_suffix(&mut self, suffix: &'static str) -> &mut Self {
        self.format.format_suffix = suffix;
        self
    }

    /// Initializes the global logger with the built env logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log
    /// events that occur before initialization will be ignored.
    ///
    /// # Errors
    ///
    /// This function will fail if it is called more than once, or if another
    /// library has already initialized a global logger.
    pub fn try_init(&mut self) -> Result<(), SetLoggerError> {
        let logger = self.build();

        let max_level = logger.filter();
        let r = log::set_boxed_logger(Box::new(logger));

        if r.is_ok() {
            log::set_max_level(max_level);
        }

        r
    }

    /// Initializes the global logger with the built env logger.
    ///
    /// This should be called early in the execution of a Rust program. Any log
    /// events that occur before initialization will be ignored.
    ///
    /// # Panics
    ///
    /// This function will panic if it is called more than once, or if another
    /// library has already initialized a global logger.
    pub fn init(&mut self) {
        self.try_init()
            .expect("Builder::init should not be called after logger initialized");
    }

    /// Build an env logger.
    ///
    /// The returned logger implements the `Log` trait and can be installed manually
    /// or nested within another logger.
    pub fn build(&mut self) -> Logger {
        assert!(!self.built, "attempt to re-use consumed builder");
        self.built = true;

        Logger {
            domain: self.log_domain,
            filter: self.filter.build(),
            writer: self.writer.build(),
            format: self.format.build(),
        }
    }

}

use crate::ohfmt::HilogFormatFn;
use crate::ohfmt::writer::HilogWriter;

pub struct Logger  {
    domain: LogDomain,
    filter: env_filter::Filter,
    writer: HilogWriter,
    format: HilogFormatFn,
}

impl Logger {
    /// Returns the maximum `LevelFilter` that this env logger instance is
    /// configured to output.
    pub fn filter(&self) -> LevelFilter {
        self.filter.filter()
    }

    fn is_loggable(&self, tag: &CStr, level: LogLevel) -> bool {
        unsafe {
            OH_LOG_IsLoggable(self.domain.0.into(), tag.as_ptr(), level)
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if ! self.enabled(record.metadata()) {
            return;
        }

        // Todo: we could write to a fixed size array on the stack, since hilog anyway has a
        // maximum supported size for tag and log.
        // Todo: I think we also need / want to split messages at newlines.

        // Log records are written to a thread-local buffer before being printed
        // to the terminal. We clear these buffers afterwards, but they aren't shrunk
        // so will always at least have capacity for the largest log record formatted
        // on that thread.
        //
        // If multiple `Logger`s are used by the same threads then the thread-local
        // formatter might have different color support. If this is the case the
        // formatter and its buffer are discarded and recreated.

        thread_local! {
                static FORMATTER: RefCell<Option<HilogFormatter>> = const { RefCell::new(None) };
            }
        
        let print = |formatter: &mut HilogFormatter, record: &Record<'_>| {
            let tag = record.module_path().and_then(|path| CString::new(path).ok())
                .unwrap_or_default();
            let _ =
                (self.format)(formatter, record).and_then(|_| formatter.print(&self.writer, record.level().into(), self.domain, tag.as_ref()));

            // Always clear the buffer afterwards
            formatter.clear();
        };

        let printed = FORMATTER
            .try_with(|tl_buf| {
                if let Ok(mut tl_buf) = tl_buf.try_borrow_mut() {
                    // There are no active borrows of the buffer
                    if let Some(ref mut formatter) = *tl_buf {
                        // We have a previously set formatter
                        print(formatter, record);
                    } else {
                        // We don't have a previously set formatter
                        let mut formatter = HilogFormatter::new(&self.writer);
                        print(&mut formatter, record);

                        *tl_buf = Some(formatter);
                    }
                } else {
                    // There's already an active borrow of the buffer (due to re-entrancy)
                    print(&mut HilogFormatter::new(&self.writer), record);
                }
            })
            .is_ok();

        if !printed {
            // The thread-local storage was not available (because its
            // destructor has already run). Create a new single-use
            // Formatter on the stack for this call.
            print(&mut HilogFormatter::new(&self.writer), record);
        }
    }

    fn flush(&self) {}
}
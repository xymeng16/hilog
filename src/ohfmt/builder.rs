use std::{io, mem};
use std::fmt::Display;
use std::io::Write;
use log::Record;
use crate::ohfmt::{HilogFormatFn, HilogFormatter, TimestampPrecision};

pub(crate) struct Builder {
    pub(crate) format_timestamp: Option<TimestampPrecision>,
    pub(crate) format_module_path: bool,
    pub(crate) format_target: bool,
    pub(crate) format_level: bool,
    pub(crate) format_indent: Option<usize>,
    pub(crate) custom_format: Option<HilogFormatFn>,
    pub(crate) format_suffix: &'static str,
    built: bool,
}

impl Builder {
    /// Convert the format into a callable function.
    ///
    /// If the `custom_format` is `Some`, then any `default_format` switches are ignored.
    /// If the `custom_format` is `None`, then a default format is returned.
    /// Any `default_format` switches set to `false` won't be written by the format.
    pub(crate) fn build(&mut self) -> HilogFormatFn {
        assert!(!self.built, "attempt to re-use consumed builder");

        let built = mem::replace(
            self,
            Builder {
                built: true,
                ..Default::default()
            },
        );

        if let Some(fmt) = built.custom_format {
            fmt
        } else {
            Box::new(move |buf, record| {
                let fmt = DefaultFormat {
                    timestamp: built.format_timestamp,
                    module_path: built.format_module_path,
                    target: built.format_target,
                    level: built.format_level,
                    written_header_value: false,
                    indent: built.format_indent,
                    suffix: built.format_suffix,
                    buf,
                };

                fmt.write(record)
            })
        }
    }
}

type SubtleStyle = &'static str;

/// The default format.
///
/// This format needs to work with any combination of crate features.
struct DefaultFormat<'a> {
    timestamp: Option<TimestampPrecision>,
    module_path: bool,
    target: bool,
    level: bool,
    written_header_value: bool,
    indent: Option<usize>,
    buf: &'a mut HilogFormatter,
    suffix: &'a str,
}

impl<'a> DefaultFormat<'a> {
    fn write(mut self, record: &Record<'_>) -> io::Result<()> {
        self.write_timestamp()?;
        self.write_level(record)?;
        self.write_module_path(record)?;
        self.write_target(record)?;
        self.finish_header()?;

        self.write_args(record)?;
        write!(self.buf, "{}", self.suffix)
    }

    fn subtle_style(&self, text: &'static str) -> SubtleStyle {
        text
    }

    fn write_header_value<T>(&mut self, value: T) -> io::Result<()>
    where
        T: Display,
    {
        if !self.written_header_value {
            self.written_header_value = true;

            let open_brace = self.subtle_style("[");
            write!(self.buf, "{}{}", open_brace, value)
        } else {
            write!(self.buf, " {}", value)
        }
    }

    fn write_level(&mut self, record: &Record<'_>) -> io::Result<()> {
        if !self.level {
            return Ok(());
        }

        let level = record.level();

        self.write_header_value(format_args!("{:<5}", level))
    }

    fn write_timestamp(&mut self) -> io::Result<()> {
        let _ = self.timestamp;
        Ok(())
    }

    fn write_module_path(&mut self, record: &Record<'_>) -> io::Result<()> {
        if !self.module_path {
            return Ok(());
        }

        if let Some(module_path) = record.module_path() {
            self.write_header_value(module_path)
        } else {
            Ok(())
        }
    }

    fn write_target(&mut self, record: &Record<'_>) -> io::Result<()> {
        if !self.target {
            return Ok(());
        }

        match record.target() {
            "" => Ok(()),
            target => self.write_header_value(target),
        }
    }

    fn finish_header(&mut self) -> io::Result<()> {
        if self.written_header_value {
            let close_brace = self.subtle_style("]");
            write!(self.buf, "{} ", close_brace)
        } else {
            Ok(())
        }
    }

    fn write_args(&mut self, record: &Record<'_>) -> io::Result<()> {
        match self.indent {
            // Fast path for no indentation
            None => write!(self.buf, "{}", record.args()),

            Some(indent_count) => {
                // Create a wrapper around the buffer only if we have to actually indent the message

                struct IndentWrapper<'a, 'b> {
                    fmt: &'a mut DefaultFormat<'b>,
                    indent_count: usize,
                }

                impl<'a, 'b> Write for IndentWrapper<'a, 'b> {
                    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                        let mut first = true;
                        for chunk in buf.split(|&x| x == b'\n') {
                            if !first {
                                write!(
                                    self.fmt.buf,
                                    "{}{:width$}",
                                    self.fmt.suffix,
                                    "",
                                    width = self.indent_count
                                )?;
                            }
                            self.fmt.buf.write_all(chunk)?;
                            first = false;
                        }

                        Ok(buf.len())
                    }

                    fn flush(&mut self) -> io::Result<()> {
                        self.fmt.buf.flush()
                    }
                }

                // The explicit scope here is just to make older versions of Rust happy
                {
                    let mut wrapper = IndentWrapper {
                        fmt: self,
                        indent_count,
                    };
                    write!(wrapper, "{}", record.args())?;
                }

                Ok(())
            }
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            format_timestamp: Some(Default::default()),
            format_module_path: false,
            format_target: true,
            format_level: true,
            format_indent: Some(4),
            custom_format: None,
            format_suffix: "\n",
            built: false,
        }
    }
}

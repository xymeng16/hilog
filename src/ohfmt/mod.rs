pub mod writer;
pub(crate) mod builder;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::{fmt, io, mem};
use std::fmt::Display;
use std::io::Write;
use std::rc::Rc;
use hilog_sys::LogLevel;
use log::Record;
use crate::{hilog_log, LogDomain};
use writer::HilogWriter;

/// Formatting precision of timestamps.
///
/// Seconds give precision of full seconds, milliseconds give thousands of a
/// second (3 decimal digits), microseconds are millionth of a second (6 decimal
/// digits) and nanoseconds are billionth of a second (9 decimal digits).
#[allow(clippy::exhaustive_enums)] // compatibility
#[derive(Copy, Clone, Debug)]
pub enum TimestampPrecision {
    /// Full second precision (0 decimal digits)
    Seconds,
    /// Millisecond precision (3 decimal digits)
    Millis,
    /// Microsecond precision (6 decimal digits)
    Micros,
    /// Nanosecond precision (9 decimal digits)
    Nanos,
}

/// The default timestamp precision is seconds.
impl Default for TimestampPrecision {
    fn default() -> Self {
        TimestampPrecision::Seconds
    }
}

pub type HilogFormatFn = Box<dyn Fn(&mut HilogFormatter, &Record<'_>) -> io::Result<()> + Sync + Send>;

pub struct HilogFormatter {
    buf: Rc<RefCell<Buffer>>,
    // writer_style is not used for Hilog
}
impl HilogFormatter {
    pub(crate) fn new(writer: &HilogWriter) -> Self {
        HilogFormatter {
            buf: Rc::new(RefCell::new(writer.buffer())),
        }
    }
    pub(crate) fn print(&self, writer: &HilogWriter, level: LogLevel, domain: LogDomain, tag: &CStr) -> io::Result<()> {
        writer.print(&self.buf.borrow(), level, domain, tag)
    }

    pub(crate) fn clear(&mut self) {
        self.buf.borrow_mut().clear();
    }
}

impl Write for HilogFormatter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf.borrow_mut().flush()
    }
}

impl fmt::Debug for HilogFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buf = self.buf.borrow();
        f.debug_struct("Formatter")
            .field("buf", &buf)
            .finish()
    }
}

#[derive(Debug, Default)]
pub(crate) struct Buffer(Vec<u8>);

impl Buffer {
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend(buf);
        Ok(buf.len())
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
use std::ffi::{CStr, CString};
use std::io;
use hilog_sys::LogLevel;
use crate::{hilog_log, LogDomain};
use crate::ohfmt::Buffer;

#[derive(Debug, Default)]
pub struct HilogWriter;

impl HilogWriter {
    pub(super) fn buffer(&self) -> Buffer {
        Buffer(Vec::new())
    }
    
    pub(super) fn print(&self, buf: &Buffer, level: LogLevel, domain: LogDomain, tag: &CStr) -> io::Result<()> {
        let c_msg = unsafe { CString::from_vec_unchecked(buf.as_bytes().to_vec()) };
        hilog_log(hilog_sys::LogType::LOG_APP, level,domain, tag, c_msg.as_ref());
        Ok(())
    }
}

#[derive(Default)]
pub struct Builder {
    built: bool,
}

impl Builder {
    pub(crate) fn build(&mut self) -> HilogWriter {
        self.built = true;
        HilogWriter
    }
}
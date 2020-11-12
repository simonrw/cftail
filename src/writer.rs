use std::fmt::Debug;
use termcolor::{ColorSpec, StandardStreamLock, WriteColor};

pub(crate) struct Writer<'a>(StandardStreamLock<'a>);

impl<'a> Writer<'a> {
    pub(crate) fn new(inner: StandardStreamLock<'a>) -> Self {
        Self(inner)
    }
}

impl<'a> Debug for Writer<'a> {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        w.write_str("writer")
    }
}

impl<'a> std::io::Write for Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl<'a> WriteColor for Writer<'a> {
    fn supports_color(&self) -> bool {
        self.0.supports_color()
    }

    fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
        self.0.set_color(spec)
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.0.reset()
    }
}

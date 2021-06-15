use std::fmt::Debug;
use termcolor::{ColorSpec, StandardStream, WriteColor};

pub(crate) struct Writer<'a> {
    stream: &'a mut StandardStream,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(stream: &'a mut StandardStream) -> Self {
        Self { stream }
    }
}

impl<'a> Debug for Writer<'a> {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        w.write_str("writer")
    }
}

impl<'a> std::io::Write for Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lock = self.stream.lock();
        lock.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut lock = self.stream.lock();
        lock.flush()
    }
}

impl<'a> WriteColor for Writer<'a> {
    fn supports_color(&self) -> bool {
        true
    }

    fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
        self.stream.set_color(spec)
    }

    fn reset(&mut self) -> std::io::Result<()> {
        self.stream.reset()
    }
}

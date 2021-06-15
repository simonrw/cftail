use std::fmt::Debug;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

pub(crate) struct Writer {
    stream: StandardStream,
}

impl Writer {
    pub(crate) fn new() -> Self {
        Self {
            stream: StandardStream::stdout(ColorChoice::Auto),
        }
    }
}

impl Debug for Writer {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        w.write_str("writer")
    }
}

impl std::io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lock = self.stream.lock();
        lock.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut lock = self.stream.lock();
        lock.flush()
    }
}

impl WriteColor for Writer {
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

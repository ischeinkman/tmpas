use std::io;

pub struct LazyWriter<W: io::Write> {
    inner: W,
    buffer: Vec<u8>,
}

impl<W: io::Write> LazyWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
        }
    }
}

impl<W: io::Write> io::Write for LazyWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.write_all(&self.buffer)?;
        self.inner.flush()?;
        self.buffer.clear();
        Ok(())
    }
}

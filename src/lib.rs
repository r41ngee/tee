use std::io::{Read, Write};

pub mod multi_writer;

pub struct Writer<W: Write>(Vec<W>);
impl<W: Write> Writer<W> {
    pub fn new() -> Self { Self(Vec::new()) }
    pub fn add_writer(&mut self, writer: W) {
        self.0.push(writer);
    }
    pub fn take(self) -> Vec<W> { self.0 }
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.0.iter_mut() {
            w.write(buf)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for w in self.0.iter_mut() {
            w.flush()?;
        }
        Ok(())
    }
}

impl<W: Write> Default for Writer<W> {
    fn default() -> Self { Self::new() }
}


pub struct Reader<R: Read, W: Write> {
    reader: R,
    writers: Writer<W>,
}

impl<R: Read, W: Write> Reader<R, W> {
    pub fn new(reader: R, writers: Writer<W>) -> Self {
        Self { reader, writers }
    }
    pub fn take(self) -> (R, Writer<W>) {
        (self.reader, self.writers)
    }
}

impl<R: Read, W: Write> Read for Reader<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)?;
        self.writers.write_all(buf)?;
        Ok(buf.len())
    }
}
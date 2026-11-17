use std::io::{Read, Write};

pub mod dynamic;
pub use dynamic::*;

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
            w.write_all(buf)?;
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
        let n = self.reader.read(buf)?;
        self.writers.write_all(&buf[..n])?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    #[test]
    fn reader_propagates_end_of_file() {
        let mut writers = Writer::new();
        writers.add_writer(Vec::<u8>::new());

        let mut tee = Reader::new(&b"abc"[..], writers);
        let mut out = Vec::new();
        tee.read_to_end(&mut out).unwrap();

        assert_eq!(out, b"abc".to_vec());

        let (_, writers) = tee.take();
        assert_eq!(writers.take()[0], b"abc".to_vec());
    }

    #[test]
    fn reader_forwards_only_bytes_that_were_read() {
        struct Trickle<'a>(&'a [u8]);

        impl<'a> Read for Trickle<'a> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = self.0.len().min(buf.len()).min(2);
                buf[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }

        let mut writers = Writer::new();
        writers.add_writer(Vec::<u8>::new());

        let mut tee = Reader::new(Trickle(b"abcdef"), writers);
        let mut out = Vec::new();

        let mut buf = [0u8; 8];
        let n = tee.read(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&buf[..n], b"ab");

        tee.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"cdef".to_vec());

        let (_, writers) = tee.take();
        assert_eq!(writers.take()[0], b"abcdef".to_vec());
    }

    #[test]
    fn writer_broadcasts_to_every_sink() {
        let mut first = Vec::new();
        let mut second = Vec::new();

        let mut writers = Writer::new();
        writers.add_writer(&mut first);
        writers.add_writer(&mut second);

        writers.write_all(b"Hello, World!").unwrap();
        writers.flush().unwrap();

        assert_eq!(first, b"Hello, World!".to_vec());
        assert_eq!(second, first);
    }

    #[test]
    fn reader_works_with_buffered_streams() {
        use std::io::{BufReader, BufWriter, Write};

        let mut backup = Vec::new();
        let mut writers = Writer::new();
        writers.add_writer(BufWriter::new(&mut backup));

        let mut tee = Reader::new(BufReader::new(Cursor::new(b"data".to_vec())), writers);
        let mut out = Vec::new();
        tee.read_to_end(&mut out).unwrap();

        assert_eq!(out, b"data".to_vec());

        let (_, mut writers) = tee.take();
        writers.flush().unwrap();
        drop(writers);

        assert_eq!(backup, out);
    }
}

use std::io::{Read, Result, Write};

/// An adapter for readers whose inputs
/// are written to a "tee"'d writer
pub struct TeeReader<'a, R: Read, W: Write> {
    reader: &'a mut R,
    writer: &'a mut W,
}

impl<'a, R: Read, W: Write> TeeReader<'a, R, W> {
    /// Returns a TeeReader which can be used as Read whose
    /// reads delegate bytes read to the provided reader and write to the provided
    /// writer. The write operation must complete before the read completes.
    ///
    /// Errors reported by the write operation will be interpreted as errors for the read
    pub fn new(reader: &'a mut R, writer: &'a mut W) -> TeeReader<'a, R, W> {
        TeeReader {
            reader: reader,
            writer: writer,
        }
    }
}

impl<R: Read, W: Write> Read for TeeReader<'_, R, W> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf)?;
        self.writer.write_all(&buf[..n])?;
        Ok(n)
    }
}


pub struct MultiTee {
    reader: Box<dyn Read>,
    writers: Vec<Box<dyn Write>>,
}

impl MultiTee {
    pub fn new(reader: impl Into<Box<dyn Read>>, writers: Vec<impl Into<Box<dyn Write>>>) -> Self {
        Self { reader: reader.into(), writers: writers.into_iter().map(|w| w.into()).collect() }
    }

    pub fn into_inner(self) -> (Box<dyn Read>, Vec<Box<dyn Write>>) {
        (self.reader, self.writers)
    }
}

impl Read for MultiTee {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf)?;
        for w in self.writers.iter_mut() {
            w.write_all(&buf[..n])?;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn tee() {
        let mut reader = "It's over 9000!".as_bytes();
        let mut teeout = Vec::new();
        let mut stdout = Vec::new();
        {
            let mut tee = TeeReader::new(&mut reader, &mut teeout);
            let _ = tee.read_to_end(&mut stdout);
        }
        assert_eq!(teeout, stdout);
    }
}

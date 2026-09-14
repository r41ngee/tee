//! An adapter for readers which delegate reads to a writer
//! ```rust
//! # use std::io::{Read, Write};
//! # use tee2::TeeReader;
//! #
//! let mut reader = "Hello, World!".as_bytes();
//! let mut writer = Vec::new();
//! let mut stdout = Vec::new();
//! 
//! let mut tee = TeeReader::new(&mut reader, &mut writer);
//! tee.read_to_end(&mut stdout).unwrap();
//! assert_eq!(writer, stdout);
//! ```

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


/// An adapter for readers whose inputs
/// are written to multiple "tee"'d writers.
pub struct MultiTee<'a> {
    reader: &'a mut dyn Read,
    writers: Vec<&'a mut dyn Write>,
}

impl<'a> MultiTee<'a> {
    /// Creates a new [`MultiTee`] instance that reads from the given reader and writes to the provided writers.
    pub fn new(reader: &'a mut dyn Read, writers: Vec<&'a mut dyn Write>) -> Self {
        Self { reader, writers }
    }
}

impl Read for MultiTee<'_> {
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

    #[test]
    fn multi_tee() {
        let mut reader = "It's over 9000!".as_bytes();
        let mut teeout1 = Vec::new();
        let mut teeout2 = Vec::new();
        let mut stdout = Vec::new();
        {
            let mut tee = MultiTee::new(&mut reader, vec![&mut teeout1, &mut teeout2]);
            let _ = tee.read_to_end(&mut stdout);
        }
        assert_eq!(teeout1, teeout2);
        assert_eq!(teeout2, stdout);
    }
}

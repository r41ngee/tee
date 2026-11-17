#![warn(missing_docs)]

//! Tee-style adapters for [`std::io`].
//!
//! A *tee* duplicates a stream: the bytes flowing through it are consumed as
//! usual and simultaneously copied somewhere else.
//!
//! This crate provides both directions:
//!
//! * [`Reader`] wraps a [`Read`] and forwards every byte it reads to one or
//!   more [`Write`] sinks.
//! * [`Writer`] is the multi-sink counterpart: it implements [`Write`] and
//!   broadcasts everything written to it to every registered writer.
//!
//! Writers are ordinary `std` types, so the copies can go to files, network
//! sockets, in-memory buffers, or any mix of them. When the sinks have
//! different types, reach for [`DynWriter`] from the [`dynamic`] module.
//!
//! # Reading
//!
//! ```rust
//! use std::io::Read;
//! use tee2::{Reader, Writer};
//!
//! // Two owned sinks, so they can be retrieved later with `take`.
//! let mut writers = Writer::new();
//! writers.add_writer(Vec::<u8>::new());
//! writers.add_writer(Vec::<u8>::new());
//!
//! let mut tee = Reader::new(&b"Hello, World!"[..], writers);
//! let mut sink = Vec::new();
//! tee.read_to_end(&mut sink).unwrap();
//!
//! let (_, writers) = tee.take();
//! let copies = writers.take();
//!
//! assert_eq!(copies[0], copies[1]);
//! assert_eq!(copies[0], b"Hello, World!");
//! ```
//!
//! # Writing
//!
//! ```rust
//! use std::io::Write;
//! use tee2::Writer;
//!
//! let mut first = Vec::new();
//! let mut second = Vec::new();
//!
//! let mut writers = Writer::new();
//! writers.add_writer(&mut first);
//! writers.add_writer(&mut second);
//!
//! writers.write_all(b"Hello, World!").unwrap();
//!
//! assert_eq!(first, second);
//! ```
//!
//! # Semantics
//!
//! * Sinks are visited sequentially in the order they were added; nothing is
//!   written in parallel, so a slow sink slows the stream down.
//! * Every sink receives the whole buffer. [`Writer::write`] uses
//!   [`Write::write_all`] internally and reports `buf.len()` on success, so a
//!   short write is never reported to the caller.
//! * If a sink fails, the error is returned immediately. Sinks visited before
//!   it already got the bytes, the ones after it did not.
//! * Sinks are not flushed on write; call [`Write::flush`] on the tee (or on
//!   [`Writer::flush`]) when the data needs to reach its destination.
//! * Nothing is buffered by the adapters: each read is copied to the sinks
//!   before the next read happens.
//!
//! [`DynWriter`]: dynamic::DynWriter

use std::io::{Read, Write};

pub mod dynamic;
pub use dynamic::*;

/// A set of [`Write`] sinks that all receive the same bytes.
///
/// `Writer` implements [`Write`] itself, so it can be used anywhere a single
/// writer is expected — for example inside a [`std::io::BufWriter`] or as the
/// destination of [`std::io::copy`].
///
/// Sinks are written to in the order they were added. Each `write` call
/// forwards the whole buffer with [`Write::write_all`] and reports
/// `buf.len()`, so partial writes are never reported to the caller.
///
/// Writers can be either owned (retrieved back with [`Writer::take`]) or
/// borrowed, which makes it easy to keep access to the buffers after the tee
/// is gone:
///
/// ```rust
/// use std::io::Write;
/// use tee2::Writer;
///
/// let mut log = Vec::new();
/// let mut archive = Vec::new();
///
/// let mut writers = Writer::new();
/// writers.add_writer(&mut log);
/// writers.add_writer(&mut archive);
///
/// writers.write_all(b"Hello, World!").unwrap();
///
/// assert_eq!(log, archive);
/// ```
///
/// If one writer fails, the error is returned immediately and the writers that
/// have not been reached yet do not receive the bytes.
pub struct Writer<W: Write>(Vec<W>);

impl<W: Write> Writer<W> {
    /// Creates an empty set of writers.
    ///
    /// Writing to an empty set succeeds and discards the bytes.
    pub fn new() -> Self { Self(Vec::new()) }

    /// Appends a writer. It will receive every subsequent write, after the
    /// writers that were added before it.
    pub fn add_writer(&mut self, writer: W) {
        self.0.push(writer);
    }

    /// Consumes the set and returns the writers in insertion order.
    pub fn take(self) -> Vec<W> { self.0 }
}

impl<W: Write> Write for Writer<W> {
    /// Writes `buf` to every registered writer with [`Write::write_all`].
    ///
    /// Returns `buf.len()` on success, or the first error encountered.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.0.iter_mut() {
            w.write_all(buf)?;
        }

        Ok(buf.len())
    }

    /// Flushes every registered writer, in insertion order.
    ///
    /// The first error encountered is returned; writers after it are not
    /// flushed.
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

/// A [`Read`] adapter that copies everything it reads into a [`Writer`].
///
/// Each successful read forwards exactly the bytes that the inner reader
/// produced (not the whole buffer) and returns the same number of bytes, so
/// the usual [`Read`] contract — including end of file — is preserved.
///
/// ```rust
/// use std::io::{Read, Write};
/// use tee2::{Reader, Writer};
///
/// let mut backup = Vec::new();
/// let mut writers = Writer::new();
/// writers.add_writer(&mut backup);
///
/// let mut tee = Reader::new(&b"Hello, World!"[..], writers);
/// let mut seen = Vec::new();
/// tee.read_to_end(&mut seen).unwrap();
///
/// assert_eq!(seen, b"Hello, World!");
/// assert_eq!(backup, seen);
/// ```
///
/// If a writer fails, the error is returned from [`Read::read`]. The bytes that
/// were already taken from the inner reader are gone; they cannot be read
/// again.
pub struct Reader<R: Read, W: Write> {
    reader: R,
    writers: Writer<W>,
}

impl<R: Read, W: Write> Reader<R, W> {
    /// Wraps `reader`, forwarding everything read from it to `writers`.
    pub fn new(reader: R, writers: Writer<W>) -> Self {
        Self { reader, writers }
    }

    /// Consumes the adapter and returns the inner reader together with the
    /// writers.
    ///
    /// ```rust
    /// use std::io::{Cursor, Read};
    /// use tee2::{Reader, Writer};
    ///
    /// let mut writers = Writer::new();
    /// writers.add_writer(Vec::<u8>::new());
    ///
    /// let mut tee = Reader::new(Cursor::new(b"hi".to_vec()), writers);
    /// let mut buf = [0; 2];
    /// tee.read(&mut buf).unwrap();
    ///
    /// let (source, writers) = tee.take();
    /// assert_eq!(source.into_inner(), b"hi".to_vec());
    /// assert_eq!(writers.take()[0], b"hi".to_vec());
    /// ```
    pub fn take(self) -> (R, Writer<W>) {
        (self.reader, self.writers)
    }
}

impl<R: Read, W: Write> Read for Reader<R, W> {
    /// Reads into `buf`, then copies the bytes that were actually read to
    /// every writer.
    ///
    /// Returns the number of bytes read from the inner reader, so `Ok(0)`
    /// still means end of file.
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

//! An adapter for writers which delegate writes to multiple writers.
//!
//! This module provides [`MultiWriter`], the write-side counterpart of
//! [`MultiTee`](crate::MultiTee): every byte written to a `MultiWriter` is
//! forwarded to all of its registered writers.
//!
//! ```rust
//! # use std::io::Write;
//! # use tee2::MultiWriter;
//! let mut first = Vec::new();
//! let mut second = Vec::new();
//!
//! let mut writer = MultiWriter::default();
//! writer.add_writer(&mut first);
//! writer.add_writer(&mut second);
//!
//! writer.write_all(b"Hello, World!").unwrap();
//!
//! assert_eq!(&first[..], b"Hello, World!");
//! assert_eq!(&second[..], b"Hello, World!");
//! ```

use std::io::Write;

/// An adapter for writers whose bytes are written to multiple "tee"'d writers.
///
/// Writers are registered with [`add_writer`](MultiWriter::add_writer) and are
/// written to in the order they were added. `MultiWriter` itself implements
/// [`Write`], so it can be used anywhere a writer is expected: as the
/// destination of [`std::io::copy`], inside a [`std::io::BufWriter`], as the
/// target of [`write!`](std::write), and so on.
///
/// Because the writers are stored as `&mut dyn Write` borrows, a `MultiWriter`
/// does not own them: they live in the caller's scope and stay usable once the
/// `MultiWriter` is dropped. A writer cannot be removed after it has been
/// added.
///
/// # Errors
///
/// Each call to [`Write::write`] forwards the whole buffer to every writer with
/// [`Write::write_all`] and then reports the full buffer length as written, so
/// a partial write is never reported. This also means that a failure in one
/// writer is *not* isolated: bytes already handed to earlier writers are gone,
/// the returning error aborts the remaining writes, and any retry of the same
/// buffer will duplicate data in the writers that already succeeded. Reserve
/// `MultiWriter` for scenarios where every sink is expected to be reliable, or
/// provide the sinks such that they cannot fail independently.
///
/// [`Write::flush`] flushes the writers in the same order and likewise stops at
/// the first error.
///
/// # Examples
///
/// Mirroring a formatted message into a log buffer and a second destination:
///
/// ```rust
/// use std::io::Write;
/// # use tee2::MultiWriter;
///
/// let mut log = Vec::new();
/// let mut mirror = Vec::new();
///
/// let mut writer = MultiWriter::default();
/// writer.add_writer(&mut log);
/// writer.add_writer(&mut mirror);
///
/// write!(writer, "value: {}", 42).unwrap();
///
/// assert_eq!(&log[..], b"value: 42");
/// assert_eq!(log, mirror);
/// ```
///
/// A `MultiWriter` can also be buffered like any other writer. The buffer has
/// to be flushed (or dropped) before the borrowed writers can be inspected
/// again:
///
/// ```rust
/// use std::io::{BufWriter, Write};
/// use tee2::MultiWriter;
///
/// let mut log = Vec::new();
/// {
///     let mut writer = MultiWriter::default();
///     writer.add_writer(&mut log);
///
///     let mut buffered = BufWriter::new(writer);
///     buffered.write_all(b"buffered").unwrap();
///     buffered.flush().unwrap();
/// }
///
/// assert_eq!(&log[..], b"buffered");
/// ```
#[derive(Default)]
pub struct MultiWriter<'a> {
    inner: Vec<&'a mut dyn Write>,
}

impl<'a> MultiWriter<'a> {
    /// Registers `writer` so that all subsequent writes and flushes are also
    /// forwarded to it.
    ///
    /// Writers already registered are unaffected; the new writer is appended
    /// to the end of the list. Registering a writer after data has been
    /// written does not replay the earlier bytes for it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io::Write;
    /// use tee2::MultiWriter;
    ///
    /// let mut first = Vec::new();
    /// let mut second = Vec::new();
    ///
    /// let mut writer = MultiWriter::default();
    /// writer.add_writer(&mut first);
    ///
    /// writer.write_all(b"hello").unwrap();
    ///
    /// writer.add_writer(&mut second);
    /// writer.write_all(b" world").unwrap();
    ///
    /// assert_eq!(&first[..], b"hello world");
    /// assert_eq!(&second[..], b" world");
    /// ```
    pub fn add_writer(&mut self, writer: &'a mut dyn Write) {
        self.inner.push(writer);
    }
}

/// Writes `buf` to every registered writer, in registration order.
///
/// See the [type-level documentation](MultiWriter) for the error semantics of
/// partial and failing writers.
impl<'a> Write for MultiWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.inner.iter_mut() {
            w.write_all(buf)?;
        }

        Ok(buf.len())
    }

    /// Flushes every registered writer, in registration order.
    ///
    /// Writers after the first one that fails are left unflushed.
    fn flush(&mut self) -> std::io::Result<()> {
        for w in self.inner.iter_mut() {
            w.flush()?;
        }
        Ok(())
    }
}

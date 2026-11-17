//! Multi-writer sets for sinks that do not share a single type.
//!
//! [`crate::Writer`] stores its writers in a `Vec<W>`, which means every sink
//! must have the same type. That is fine when the copies go to two `Vec<u8>`
//! buffers, but not when one copy goes to a file and another to a socket.
//!
//! [`DynWriter`] solves this by keeping type-erased writers:
//!
//! * [`DynWriter<Box<dyn Write>>`] owns its sinks. Build one with
//!   [`DynWriter::new_boxed`] or [`DynWriter::add_boxed`].
//! * [`DynWriter<&mut dyn Write>`] borrows its sinks, which is handy when the
//!   caller wants to look at the buffers afterwards.
//!
//! Both implement [`Write`] and follow the same rules as [`crate::Writer`]:
//! sinks are visited in insertion order, each one gets the whole buffer via
//! [`Write::write_all`], `buf.len()` is reported on success, and the first
//! error aborts the remaining writes.
//!
//! # Borrowing heterogeneous writers
//!
//! ```rust
//! use std::io::Write;
//! use tee2::DynWriter;
//!
//! let mut log = Vec::new();
//! let mut archive = Vec::new();
//!
//! let mut writers: DynWriter<&mut dyn Write> = DynWriter::default();
//! writers.add_writer(&mut log);
//! writers.add_writer(&mut archive);
//!
//! writers.write_all(b"Hello, World!").unwrap();
//! drop(writers);
//!
//! assert_eq!(log, archive);
//! ```
//!
//! # Owning heterogeneous writers
//!
//! Once boxed, a sink can only be reached through the trait object, so shared
//! state is the usual way to observe the data afterwards:
//!
//! ```rust
//! use std::io::{self, Write};
//! use std::sync::{Arc, Mutex};
//! use tee2::DynWriter;
//!
//! #[derive(Clone, Default)]
//! struct Shared(Arc<Mutex<Vec<u8>>>);
//!
//! impl Write for Shared {
//!     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//!         self.0.lock().unwrap().extend_from_slice(buf);
//!         Ok(buf.len())
//!     }
//!
//!     fn flush(&mut self) -> io::Result<()> {
//!         Ok(())
//!     }
//! }
//!
//! let sink = Shared::default();
//!
//! let mut writers = DynWriter::new_boxed(vec![sink.clone()]);
//! writers.add_boxed(std::io::sink());
//! writers.write_all(b"Hello, World!").unwrap();
//!
//! assert_eq!(*sink.0.lock().unwrap(), b"Hello, World!".to_vec());
//! ```

use std::io::Write;
use std::ops::DerefMut;

type BoxW = Box<dyn Write + 'static>;

/// A set of type-erased [`Write`] sinks that all receive the same bytes.
///
/// `B` is any type that dereferences to `dyn Write`, most commonly
/// `Box<dyn Write>` (owning) or `&mut dyn Write` (borrowing).
///
/// See the [module documentation](self) for examples and error semantics.
#[derive(Default)]
pub struct DynWriter<B>
where
    B: DerefMut<Target = dyn Write>
{
    inner: Vec<B>,
}

impl<B> DynWriter<B>
where
    B: DerefMut<Target = dyn Write>
{
    /// Creates a set from already type-erased writers.
    pub fn new(writers: Vec<B>) -> Self {
        Self { inner: writers }
    }

    /// Appends a writer. It will receive every subsequent write, after the
    /// writers that were added before it.
    pub fn add_writer(&mut self, writer: B) {
        self.inner.push(writer);
    }

    /// Consumes the set and returns the writers in insertion order.
    pub fn take(self) -> Vec<B> {
        self.inner
    }
}

impl DynWriter<BoxW> {
    /// Boxes each writer and creates a set from them.
    ///
    /// ```rust
    /// use std::io::Write;
    /// use tee2::DynWriter;
    ///
    /// let mut writers = DynWriter::new_boxed(vec![Vec::<u8>::new()]);
    /// writers.write_all(b"hi").unwrap();
    ///
    /// assert_eq!(writers.take().len(), 1);
    /// ```
    pub fn new_boxed<W: Write + 'static>(writers: Vec<W>) -> Self {
        Self::new(
            writers
                .into_iter()
                .map(|w| {
                    Box::new(w) as BoxW
                })
                .collect()
        )
    }

    /// Boxes `writer` and appends it to the set.
    ///
    /// ```rust
    /// use std::io::Write;
    /// use tee2::DynWriter;
    ///
    /// let mut writers = DynWriter::default();
    /// writers.add_boxed(Vec::<u8>::new());
    /// writers.write_all(b"hi").unwrap();
    ///
    /// assert_eq!(writers.take().len(), 1);
    /// ```
    pub fn add_boxed<W: Write + 'static>(&mut self, writer: W) {
        self.inner.push(Box::new(writer) as BoxW);
    }
}

impl<B> Write for DynWriter<B>
where
    B: DerefMut<Target = dyn Write>,
{
    /// Writes `buf` to every registered writer with [`Write::write_all`].
    ///
    /// Returns `buf.len()` on success, or the first error encountered.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.inner.iter_mut() {
            w.write_all(buf)?;
        }
        Ok(buf.len())
    }

    /// Flushes every registered writer, in insertion order.
    ///
    /// The first error encountered is returned; writers after it are not
    /// flushed.
    fn flush(&mut self) -> std::io::Result<()> {
        for w in self.inner.iter_mut() {
            w.flush()?;
        }
        Ok(())
    }
}

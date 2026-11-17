# Tee2

![Crates.io Version](https://img.shields.io/crates/v/tee2)
<br>
![docs.rs](https://img.shields.io/docsrs/tee2)
![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/tee2/latest)
![Crates.io MSRV](https://img.shields.io/crates/msrv/tee2)

Tee-style adapters for Rust I/O.

A *tee* duplicates a stream: the data is consumed as usual **and** copied
somewhere else. `tee2` gives you both directions:

| Type | Direction | What it does |
| --- | --- | --- |
| [`Reader`] | reading | wraps any `Read`, copies every byte it reads into one or more `Write` sinks |
| [`Writer`] | writing | implements `Write`, broadcasts everything written to it to every registered writer |
| [`DynWriter`] | writing | same as `Writer`, but the sinks may have different types |

Typical uses: logging a request body while parsing it, writing a file and a
hash at the same time, mirroring a stream to `stdout` for debugging, or keeping
a backup copy of data that came off a socket.

## Installation

```toml
[dependencies]
tee2 = "1.0"
```

## Reading

`Reader` forwards exactly the bytes that the inner reader produced, so the usual
`Read` contract — including end of file — is preserved.

```rust
use std::io::Read;
use tee2::{Reader, Writer};

let mut writers = Writer::new();
writers.add_writer(Vec::<u8>::new());
writers.add_writer(Vec::<u8>::new());

let mut tee = Reader::new(&b"Hello, World!"[..], writers);
let mut sink = Vec::new();
tee.read_to_end(&mut sink).unwrap();

let (_, writers) = tee.take();
let copies = writers.take();

assert_eq!(copies[0], copies[1]);
assert_eq!(copies[0], b"Hello, World!");
```

Writers can also be borrowed, which keeps them accessible after the tee is
dropped:

```rust
use std::io::Write;
use tee2::{Reader, Writer};

let mut backup = Vec::new();
let mut writers = Writer::new();
writers.add_writer(&mut backup);

let mut tee = Reader::new(&b"Hello, World!"[..], writers);
let mut seen = Vec::new();
tee.read_to_end(&mut seen).unwrap();
drop(tee);

assert_eq!(backup, seen);
```

`Reader::take` gives back the inner reader together with the writers, which is
how you reclaim owned sinks or recover state after reading.

## Writing

`Writer` is a `Write` implementation that forwards every write to all of its
writers, in insertion order:

```rust
use std::io::Write;
use tee2::Writer;

let mut log = Vec::new();
let mut archive = Vec::new();

let mut writers = Writer::new();
writers.add_writer(&mut log);
writers.add_writer(&mut archive);

writers.write_all(b"Hello, World!").unwrap();

assert_eq!(log, archive);
```

Because `Writer` implements `Write`, it composes with the rest of `std::io`:
wrap it in a `BufWriter`, pass it to `std::io::copy`, or use it as the
destination of any function that expects a writer.

## Heterogeneous sinks

`Writer` stores its sinks in a `Vec<W>`, so they all share one type. When the
copies go to different kinds of destinations, use `DynWriter` from
`tee2::dynamic`, which keeps type-erased writers.

Borrowed sinks (`DynWriter<&mut dyn Write>`):

```rust
use std::io::Write;
use tee2::DynWriter;

let mut file = Vec::new();
let mut console = Vec::new();

let mut writers: DynWriter<&mut dyn Write> = DynWriter::default();
writers.add_writer(&mut file);
writers.add_writer(&mut console);

writers.write_all(b"Hello, World!").unwrap();
drop(writers);

assert_eq!(file, console);
```

Owned sinks (`DynWriter<Box<dyn Write>>`), created with `new_boxed` /
`add_boxed`:

```rust
use std::io::Write;
use tee2::DynWriter;

let mut writers = DynWriter::default();
writers.add_boxed(Vec::<u8>::new());
writers.add_boxed(std::io::sink());

writers.write_all(b"Hello, World!").unwrap();
assert_eq!(writers.take().len(), 2);
```

`DynWriter` implements `Write` too, so it can be nested inside a `Writer` when
you want a [`Reader`] to fan out to sinks of different types.

## Semantics

* **Order.** Sinks are visited sequentially in the order they were added;
  nothing is written in parallel, so a slow sink slows the whole stream down.
* **Complete writes.** Each `write` call forwards the whole buffer with
  `write_all` and reports `buf.len()`, so partial writes are never reported to
  the caller.
* **Errors.** If a sink fails, the error is returned immediately. Sinks visited
  before it already received the bytes, the ones after it did not — and bytes
  already taken from the inner reader are gone.
* **Flushing.** The adapters never flush implicitly; call `flush()` on the tee
  (which flushes every sink in order) when the data must reach its destination.
* **No buffering.** Data is copied to the sinks before the next read or write
  happens; the adapters add no buffering of their own.
* **Empty set.** Writing to a set with no writers succeeds and discards the
  bytes.

## MSRV

Rust 1.32.0 (edition 2018).

## License

MIT — see [LICENSE](LICENSE).

## Credits

Originally written by Doug Tangren (softprops) in 2015 as
[`tee`](https://github.com/softprops/tee); this fork is maintained by
[r41ngee](https://github.com/r41ngee) and published as `tee2`.

[`Reader`]: https://docs.rs/tee2/latest/tee2/struct.Reader.html
[`Writer`]: https://docs.rs/tee2/latest/tee2/struct.Writer.html
[`DynWriter`]: https://docs.rs/tee2/latest/tee2/dynamic/struct.DynWriter.html

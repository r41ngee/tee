# Tee2

![Crates.io Version](https://img.shields.io/crates/v/tee2)
<br>
![docs.rs](https://img.shields.io/docsrs/tee2)
![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/tee2/latest)

A rustlang adapter for readers which delegate read bytes to a writer.

## Examples

```rust
use std::io::{Read, Write};
use tee2::TeeReader;

let mut reader = "Hello, World!".as_bytes();
let mut writer = Vec::new();
let mut stdout = Vec::new();

let mut tee = TeeReader::new(&mut reader, &mut writer);
tee.read_to_end(&mut stdout).unwrap();
assert_eq!(writer, stdout);
```

## Writing to multiple writers

`MultiWriter` is the write-side counterpart of `MultiTee`: anything written to
it is forwarded to every registered writer, in the order the writers were
added.

```rust
use std::io::Write;
use tee2::MultiWriter;

let mut stdout = Vec::new();
let mut file = Vec::new();

let mut writer = MultiWriter::default();
writer.add_writer(&mut stdout);
writer.add_writer(&mut file);

writer.write_all(b"Hello, World!").unwrap();

assert_eq!(stdout, file);
```

A `MultiWriter` implements `Write`, so it can be passed anywhere a writer is
expected, including into `BufWriter` or as the destination of `std::io::copy`.

Each `write` call forwards the whole buffer with `write_all` and reports the
full length as written, so partial writes are never reported. If one writer
fails, the error is returned immediately: writers visited before it already
received the bytes, the ones after it did not.

Doug Tangren (softprops) 2015<br>
r41ngee 2026

Forked from [https://github.com/softprops/tee]

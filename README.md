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

Doug Tangren (softprops) 2015<br>
r41ngee 2026

Forked from [https://github.com/softprops/tee]

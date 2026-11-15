use std::io::Write;

#[derive(Default)]
pub struct MultiWriter<'a> {
    inner: Vec<&'a mut dyn Write>,
}

impl<'a> MultiWriter<'a> {
    pub fn add_writer(&mut self, writer: &'a mut dyn Write) {
        self.inner.push(writer);
    }
}

impl<'a> Write for MultiWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.inner.iter_mut() {
            w.write_all(buf)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for w in self.inner.iter_mut() {
            w.flush()?;
        }
        Ok(())
    }
}
use std::io::Write;
use std::ops::DerefMut;

type BoxW = Box<dyn Write + 'static>;

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
    pub fn new(writers: Vec<B>) -> Self {
        Self { inner: writers }
    }
    pub fn add_writer(&mut self, writer: B) {
        self.inner.push(writer);
    }
}

impl DynWriter<BoxW> {
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
    pub fn add_boxed<W: Write + 'static>(&mut self, writer: W) {
        self.inner.push(Box::new(writer) as BoxW);
    }
}

impl<B> Write for DynWriter<B>
where
    B: DerefMut<Target = dyn Write>,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for w in self.inner.iter_mut() {
            w.write(buf)?;
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
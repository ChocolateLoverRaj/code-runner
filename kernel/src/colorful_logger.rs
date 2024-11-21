use core::fmt::Write;

use log::{Level, Log};

use crate::set_color::SetColor;

pub type GetColor<C> = fn(Level) -> C;

pub struct ColorfulLogger<C, W: Write + SetColor<C>> {
    colorful_writer: spin::Mutex<W>,
    get_color: GetColor<C>,
}

impl<C, W: Write + SetColor<C>> ColorfulLogger<C, W> {
    pub fn new(colorful_writer: W, get_color: GetColor<C>) -> Self {
        Self {
            colorful_writer: spin::Mutex::new(colorful_writer),
            get_color,
        }
    }
}

impl<C, W: Write + SetColor<C> + Send> Log for ColorfulLogger<C, W>
where
    C: Send + Sync,
{
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut colorful_writer = self.colorful_writer.lock();
        colorful_writer.set_color((self.get_color)(record.level()));
        writeln!(colorful_writer, "{:5} {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}

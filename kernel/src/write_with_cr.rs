use core::fmt::Write;

/// A writer that writes to a writer but replaces `\n` with `\r\n`
pub struct WriterWithCr<T> {
    writer: T,
}

impl<T> WriterWithCr<T> {
    pub const fn new(writer: T) -> Self {
        Self { writer }
    }
}

impl<T: Write> Write for WriterWithCr<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let s = s.as_bytes();
        let mut i = 0;
        while let Some(char) = s.get(i) {
            match char {
                b'\r' => {
                    self.writer.write_char(*char as char)?;
                    i += 1;
                    match s.get(i) {
                        Some(char) => match char {
                            b'\n' => {}
                            char => self.writer.write_char(*char as char)?,
                        },
                        None => break,
                    }
                }
                b'\n' => self.writer.write_str("\r\n")?,
                char => self.writer.write_char(*char as char)?,
            };
            i += 1;
        }
        Ok(())
    }
}

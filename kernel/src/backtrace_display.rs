use core::fmt::Display;

use alloc::{string::String, vec::Vec};

#[derive(Debug)]
pub struct AtLeastLineNumber {
    pub line_number: u32,
    pub column_number: Option<u32>,
}

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub line_number: Option<AtLeastLineNumber>,
}

#[derive(Debug)]
pub struct BacktraceEntry {
    pub address: u64,
    pub function_name: Option<String>,
    pub file: Option<FileInfo>,
}

/// Prints a backtrace, just like the `std` one
pub struct BacktraceDisplay {
    entries: Vec<BacktraceEntry>,
}

impl BacktraceDisplay {
    pub fn new(entries: Vec<BacktraceEntry>) -> Self {
        Self { entries }
    }
}

impl Display for BacktraceDisplay {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "stack backtrace:");
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                write!(f, "   {}: ", index);
                if let Some(function_name) = &entry.function_name {
                    write!(f, "{}", function_name)?;
                } else {
                    write!(f, "0x{:X}", entry.address)?;
                }
                writeln!(f)?;
                write!(f, "          at ")?;
                if let Some(file) = &entry.file {
                    write!(f, "{}", file.name);
                    if let Some(line) = &file.line_number {
                        write!(f, ":{}", line.line_number)?;
                        if let Some(column) = line.column_number {
                            write!(f, ":{}", column)?;
                        }
                    }
                } else {
                    write!(f, "<unknown>")?;
                }
                writeln!(f)?;
                Ok::<_, core::fmt::Error>(())
            })
            .collect::<core::fmt::Result>()?;
        Ok(())
    }
}

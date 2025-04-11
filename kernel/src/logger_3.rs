use core::{
    fmt::{Display, Write},
    ops::DerefMut,
    ptr::NonNull,
    slice,
};

use acpi::{
    address::AddressSpace,
    spcr::{Spcr, SpcrInterfaceType},
};
use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, Styled, StyledDrawable},
    text::{Baseline, Text},
};
use limine::{framebuffer::Framebuffer, response::FramebufferResponse};
use log::{Level, Log};
use owo_colors::{AnsiColors, OwoColorize};
use spinning_top::Spinlock;
use uart_16550::{
    mmio::MemoryMappedRegister,
    port::PortAccessedRegister,
    uart_16550::{Uart16550, Uart16550Registers},
};
use unicode_segmentation::UnicodeSegmentation;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{FrameAllocator, Mapper, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    config::CONFIG, find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table, hhdm_offset::HhdmOffset,
    limine_frame_buffer_embedded_graphics::LimineFrameBufferEmbeddedGraphics,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
};

static LOGGER: Logger3 = Logger3::initial_logger();

enum SerialPort {
    Com1(Uart16550Registers<PortAccessedRegister>),
    Mmio(Uart16550Registers<MemoryMappedRegister<'static>>),
}

impl SerialPort {
    fn get_dyn_mut(&mut self) -> &mut dyn Uart16550 {
        match self {
            SerialPort::Com1(registers) => registers,
            SerialPort::Mmio(registers) => registers,
        }
    }
}

struct FrameBufferInfo {
    hhdm_offset: HhdmOffset,
    frame_buffer: LimineFrameBufferEmbeddedGraphics<'static>,
    y_position: i32,
}

struct Logger3Data {
    frame_buffer: Option<FrameBufferInfo>,
    serial_port: Option<SerialPort>,
}

struct Logger3 {
    data: Spinlock<Logger3Data>,
}

impl Logger3 {
    const fn initial_logger() -> Self {
        Self {
            data: Spinlock::new(Logger3Data {
                frame_buffer: None,
                serial_port: None,
            }),
        }
    }
}

impl Log for Logger3 {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        todo!()
    }

    fn log(&self, record: &log::Record) {
        let mut data = self.data.lock();
        if let Some(serial_port) = &mut data.serial_port {
            let writer = serial_port.get_dyn_mut();
            let level = record.level();
            let level: &dyn Display = if CONFIG.kernel_log_serial_colors {
                &level.color(match level {
                    Level::Error => AnsiColors::Red,
                    Level::Warn => AnsiColors::Yellow,
                    Level::Info => AnsiColors::Blue,
                    Level::Debug => AnsiColors::Green,
                    Level::Trace => AnsiColors::Cyan,
                })
            } else {
                &level
            };
            writeln!(writer, "{:5}: {}", level, record.args()).unwrap();
        }
        if let Some(frame_buffer_data) = &mut data.frame_buffer {
            let font = CONFIG.kernel_log_screen.unwrap();
            // The only way to iter &str from format args is to implement `Writer` and use `write!` macro.
            struct Writer<'a, 'b> {
                font: &'a MonoFont<'a>,
                display: &'a mut LimineFrameBufferEmbeddedGraphics<'b>,
                position: &'a mut Point,
                text_color: <LimineFrameBufferEmbeddedGraphics<'a> as DrawTarget>::Color,
                background_color: <LimineFrameBufferEmbeddedGraphics<'a> as DrawTarget>::Color,
            }
            impl Write for Writer<'_, '_> {
                fn write_str(&mut self, s: &str) -> core::fmt::Result {
                    for c in s.graphemes(true) {
                        let height_not_seen = self.position.y
                            + self.font.character_size.height as i32
                            - self.display.bounding_box().size.height as i32;
                        if height_not_seen > 0 {
                            self.display.shift_up(height_not_seen as u32);
                            self.position.y -= height_not_seen;
                        }
                        if c == "\r" {
                            // We don't do that here
                            continue;
                        } else if c == "\n" || c == "\r\n" {
                            // Fill the remaining space with background color
                            Rectangle::new(
                                *self.position,
                                Size::new(
                                    self.display.bounding_box().size.width - self.position.x as u32,
                                    self.font.character_size.height,
                                ),
                            )
                            .into_styled(
                                PrimitiveStyleBuilder::new()
                                    .fill_color(self.background_color)
                                    .build(),
                            )
                            .draw(self.display)
                            .map_err(|_| core::fmt::Error)?;
                            self.position.y += self.font.character_size.height as i32;
                            self.position.x = 0;
                        } else {
                            let style = MonoTextStyleBuilder::new()
                                .font(self.font)
                                .text_color(self.text_color)
                                .background_color(self.background_color)
                                .build();
                            *self.position =
                                Text::with_baseline(c, *self.position, style, Baseline::Top)
                                    .draw(self.display)
                                    .map_err(|_| core::fmt::Error)?;
                            if self.position.x as u32 + self.font.character_size.width
                                > self.display.bounding_box().size.width
                            {
                                self.position.y += self.font.character_size.height as i32;
                                self.position.x = 0;
                            }
                        }
                    }
                    Ok(())
                }
            }

            let mut writer = Writer {
                font: &font,
                display: &mut frame_buffer_data.frame_buffer,
                position: &mut Point::new(0, frame_buffer_data.y_position as i32),
                text_color: match record.level() {
                    Level::Error => Rgb888::RED,
                    Level::Warn => Rgb888::YELLOW,
                    Level::Info => Rgb888::BLUE,
                    Level::Debug => Rgb888::GREEN,
                    Level::Trace => Rgb888::CYAN,
                },
                background_color: Rgb888::BLACK,
            };
            write!(writer, "{:5}", record.level());
            writer.text_color = Rgb888::WHITE;
            writeln!(writer, ": {}", record.args()).unwrap();
            frame_buffer_data.y_position = writer.position.y;
        }
    }

    fn flush(&self) {
        // Nothing to flush
    }
}

pub fn init(frame_buffer_response: Option<&'static FramebufferResponse>, hhdm_offset: HhdmOffset) {
    let mut data = LOGGER.data.lock();
    if CONFIG.kernel_log_com1 {
        data.serial_port = Some(SerialPort::Com1({
            // Safety: This is COM1 and we have permission to access it.
            // It never hurts to write to COM1 even on computers where it doesn't do anything.
            unsafe { uart_16550::port::new(0x3F8) }
        }));
    }
    if CONFIG.kernel_log_screen.is_some() {
        if let Some(frame_buffer_response) = frame_buffer_response {
            if let Some(frame_buffer) = frame_buffer_response.framebuffers().next() {
                data.frame_buffer = Some(FrameBufferInfo {
                    hhdm_offset,
                    frame_buffer: LimineFrameBufferEmbeddedGraphics::try_from(frame_buffer)
                        .unwrap(),
                    y_position: Default::default(),
                });
            };
        }
    }

    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(CONFIG.kernel_log_serial_level_filter);
}

/// Returns the start address and stride
fn get_16550_compatible_mmio(spcr: &Spcr) -> Option<(PhysAddr, usize)> {
    match spcr.interface_type() {
        SpcrInterfaceType::Full16550
        | SpcrInterfaceType::Full16450
        | SpcrInterfaceType::Generic16550 => Some(()),
        _ => None,
    }?;
    let base_address = spcr.base_address()?.ok()?;
    match base_address.address_space {
        AddressSpace::SystemMemory => Some(()),
        _ => None,
    }?;
    match base_address.bit_offset {
        0 => Some(()),
        _ => None,
    }?;
    Some((
        PhysAddr::new(base_address.address),
        (base_address.bit_width / 8) as usize,
    ))
}

pub fn init_spcr(
    spcr: &Spcr,
    hhdm_offset: HhdmOffset,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    if let Some((base_address, stride)) = get_16550_compatible_mmio(spcr) {
        // Map the page
        // Assume that the base address is aligned
        if stride * 8 > 0x1000 {
            todo!();
        }
        let physical_frame = PhysFrame::<Size4KiB>::from_start_address(base_address).unwrap();
        let mut offset_page_table = get_offset_page_table(hhdm_offset);
        let page = find_contiguous_unused_virtual_memory(
            unsafe { PageTablesRecursiveIterator::new(hhdm_offset, Cr3::read().0, 256) },
            1,
        )
        .unwrap()
        .start;
        unsafe {
            offset_page_table.map_to(
                page,
                physical_frame,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::NO_CACHE
                    | PageTableFlags::WRITE_THROUGH,
                frame_allocator,
            )
        }
        .unwrap()
        .flush();
        let mut uart = unsafe {
            uart_16550::mmio::new(
                NonNull::new(page.start_address().as_mut_ptr()).unwrap(),
                stride,
            )
        };
        // Baud rate for Chromebooks: 115200
        // TODO: Determine baud rate for computers that are not Chromebooks
        uart.init_with_dl(0x01, 0x00);
        LOGGER.data.lock().serial_port = if CONFIG.kernel_log_spcr {
            Some(SerialPort::Mmio(uart))
        } else {
            None
        };
    };
}

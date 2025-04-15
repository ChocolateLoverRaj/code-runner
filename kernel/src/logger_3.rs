use core::{
    fmt::{Arguments, Debug, Display, Write},
    ptr::NonNull,
};

use acpi::{
    address::AddressSpace,
    spcr::{Spcr, SpcrInterfaceType},
};
use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyleBuilder},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use limine::response::FramebufferResponse;
use log::{Level, LevelFilter, Log};
use owo_colors::OwoColorize;
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
    config::{LogSerialConfig, CONFIG},
    cpu_local_data,
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
    limine_frame_buffer_embedded_graphics::LimineFrameBufferEmbeddedGraphics,
    mutex_without_interrupts::LockWithoutInterrupts,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
};

static LOGGER: Logger3 = Logger3::initial_logger();

enum SerialPort {
    Com1(Uart16550Registers<PortAccessedRegister>),
    Mmio(Uart16550Registers<MemoryMappedRegister<'static>>),
}

impl SerialPort {
    const fn new_com1() -> Self {
        SerialPort::Com1({
            // Safety: This is COM1 and we have permission to access it.
            // It never hurts to write to COM1 even on computers where it doesn't do anything.
            unsafe { uart_16550::port::new(0x3F8) }
        })
    }

    fn get_dyn_mut(&mut self) -> &mut dyn Uart16550 {
        match self {
            SerialPort::Com1(registers) => registers,
            SerialPort::Mmio(registers) => registers,
        }
    }
}

struct FrameBufferData {
    frame_buffer: LimineFrameBufferEmbeddedGraphics<'static>,
    position: Point,
}

struct Logger3Data {
    frame_buffer: Option<FrameBufferData>,
    serial_port: Option<SerialPort>,
}

struct Logger3 {
    data: LockWithoutInterrupts<Logger3Data>,
}

impl Logger3 {
    const fn initial_logger() -> Self {
        Self {
            data: LockWithoutInterrupts::new(Logger3Data {
                frame_buffer: None,
                serial_port: None,
            }),
        }
    }
}

enum LoggerColor {
    Message,
    Cpu,
    LogLevel(Level),
}

trait WriteColored {
    fn write_colored(&mut self, args: Arguments, color: LoggerColor) -> core::fmt::Result;
}

impl WriteColored for SerialPort {
    fn write_colored(&mut self, args: Arguments, color: LoggerColor) -> core::fmt::Result {
        let writer = self.get_dyn_mut();
        let args: &dyn Display = match color {
            LoggerColor::Message => &args,
            LoggerColor::LogLevel(level) => match level {
                Level::Error => &args.red(),
                Level::Warn => &args.yellow(),
                Level::Info => &args.blue(),
                Level::Debug => &args.green(),
                Level::Trace => &args.cyan(),
            },
            LoggerColor::Cpu => &args.magenta(),
        };
        write!(writer, "{}", args)
    }
}

impl WriteColored for FrameBufferData {
    fn write_colored(&mut self, args: Arguments, color: LoggerColor) -> core::fmt::Result {
        let font = CONFIG.kernel_log_screen.unwrap().font;
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
                    let height_not_seen = self.position.y + self.font.character_size.height as i32
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
            display: &mut self.frame_buffer,
            position: &mut self.position,
            text_color: match color {
                LoggerColor::Message => Rgb888::WHITE,
                LoggerColor::LogLevel(color) => match color {
                    Level::Error => Rgb888::RED,
                    Level::Warn => Rgb888::YELLOW,
                    Level::Info => Rgb888::BLUE,
                    Level::Debug => Rgb888::GREEN,
                    Level::Trace => Rgb888::CYAN,
                },
                LoggerColor::Cpu => Rgb888::MAGENTA,
            },
            background_color: Rgb888::BLACK,
        };
        let r = write!(writer, "{}", args);
        r
    }
}

fn log_record(writer: &mut impl WriteColored, record: &log::Record) -> core::fmt::Result {
    struct CpuIdDebug;
    impl Debug for CpuIdDebug {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            if let Some(data) = cpu_local_data::get_local() {
                f.write_fmt(format_args!("CPU 0x{:02X}", data.cpu_id))
            } else {
                f.write_str("BSP")
            }
        }
    }
    writer.write_colored(format_args!("[{:?}]", CpuIdDebug), LoggerColor::Cpu)?;
    writer.write_colored(
        format_args!(" {:5}", record.level()),
        LoggerColor::LogLevel(record.level()),
    )?;
    writer.write_colored(format_args!(": {}\n", record.args()), LoggerColor::Message)?;
    Ok(())
}

impl Log for Logger3 {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        todo!()
    }

    fn log(&self, record: &log::Record) {
        self.data.lock_without_interrupts(|data| {
            if let Some(serial_port) = &mut data.serial_port {
                let log_serial_config = CONFIG.kernel_log_serial.unwrap();
                if record.level() <= log_serial_config.level_filter {
                    log_record(serial_port, record).unwrap();
                }
            }
            if let Some(frame_buffer_data) = &mut data.frame_buffer {
                let log_screen_config = CONFIG.kernel_log_screen.unwrap();
                if record.level() <= log_screen_config.level_filter {
                    log_record(frame_buffer_data, record).unwrap();
                }
            }
        });
    }

    fn flush(&self) {
        // Nothing to flush
    }
}

pub fn init(frame_buffer_response: Option<&'static FramebufferResponse>) {
    LOGGER.data.lock_without_interrupts(|data| {
        if let Some(serial) = CONFIG.kernel_log_serial {
            if serial.always_log_com1 {
                data.serial_port = Some(SerialPort::new_com1());
            }
        }
        if CONFIG.kernel_log_screen.is_some() {
            if let Some(frame_buffer_response) = frame_buffer_response {
                if let Some(frame_buffer) = frame_buffer_response.framebuffers().next() {
                    data.frame_buffer = Some(FrameBufferData {
                        frame_buffer: LimineFrameBufferEmbeddedGraphics::try_from(frame_buffer)
                            .unwrap(),
                        position: Default::default(),
                    });
                };
            }
        }

        log::set_logger(&LOGGER).unwrap();
        log::set_max_level({
            let mut level_filter = LevelFilter::Off;
            if let Some(config) = CONFIG.kernel_log_serial {
                level_filter = level_filter.max(config.level_filter);
            }
            if let Some(config) = CONFIG.kernel_log_screen {
                level_filter = level_filter.max(config.level_filter);
            }
            level_filter
        });
    });
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
    spcr: Option<&Spcr>,
    log_serial_config: &LogSerialConfig,
    hhdm_offset: HhdmOffset,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    LOGGER.data.lock_without_interrupts(|data| {
        if let Some(spcr) = spcr {
            if let Some((base_address, stride)) = get_16550_compatible_mmio(spcr) {
                // Map the page
                // Assume that the base address is aligned
                if stride * 8 > 0x1000 {
                    todo!();
                }
                let physical_frame =
                    PhysFrame::<Size4KiB>::from_start_address(base_address).unwrap();
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
                data.serial_port = Some(SerialPort::Mmio(uart));
                return;
            };
        }
        if !log_serial_config.always_log_com1 {
            data.serial_port = Some(SerialPort::new_com1());
        }
    });
}

/// # Safety
/// This function is always unsafe. Only call it during a panic handler.
pub unsafe fn force_unlock() {
    unsafe { LOGGER.data.force_unlock() };
}

/// Writes to all log outputs
pub fn write_all(args: Arguments) -> core::fmt::Result {
    LOGGER.data.lock_without_interrupts(|data| {
        if let Some(serial_port) = &mut data.serial_port {
            serial_port.write_colored(args, LoggerColor::Message)?;
        }
        if let Some(frame_buffer_data) = &mut data.frame_buffer {
            frame_buffer_data.write_colored(args, LoggerColor::Message)?;
        }
        Ok(())
    })
}

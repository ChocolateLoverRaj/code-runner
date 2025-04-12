use crate::{
    call_stack_iterator::CallStackIterator, cpu_local_data::get_local, hlt_loop::hlt_loop,
    limine_requests::EXECUTABLE_FILE_REQUEST, logger_3,
};
use addr2line::{
    gimli::{Dwarf, DwarfFileType, EndianSlice, LittleEndian},
    Location,
};
use core::{
    fmt::{Debug, Display},
    panic::PanicInfo,
    slice,
};
use elf::{endian::NativeEndian, ElfBytes, ParseError};
use thiserror::Error;
use x2apic::lapic::IpiAllShorthand;
use x86_64::instructions::interrupts;

// #[cfg(not(test))]
#[panic_handler]
fn kernel_panic_handler(info: &PanicInfo) -> ! {
    // If we don't disable interrupts, code could run while we are in an invalid state. We are in an invalid state from now until reboot because of the panic.
    interrupts::disable();

    if let Some(cpu_local_data) = get_local() {
        if let Ok(local_apic) = cpu_local_data.local_apic.try_get() {
            // Safety: We need to send an NMI, regardless of whatever is holding the lock to the Local APIC
            unsafe { local_apic.force_unlock() };
            // Safety: The NMI handlers will halt the other CPUs
            // TODO: If the other CPUs have started initializing but did not set the NMI handler yet, we might triple fault. Idk if this is worth fixing though cuz we will only panic if there is a bug in the kernel and the chances of there being a bug that happens right during this timing is very low.
            unsafe {
                local_apic
                    .lock()
                    .send_nmi_all(IpiAllShorthand::AllExcludingSelf)
            };
        }
    }

    // Safety: We are already in an undefined state and we just need to simply log the panic information. We will not be logging any more messages.
    unsafe {
        logger_3::force_unlock();
    }
    // To make sure there is a new line before the panic message
    log::error!("Kernel panicked");

    // Print the backtrace
    #[derive(Debug, Error)]
    enum GetLineError {
        #[error("Limine did not have a response for the executable file request")]
        NoExecutableFileResponse,
        #[error("Error parsing kernel's ELF file")]
        ParseElfError(elf::ParseError),
        #[error("Error constructing addr2line Context")]
        Addr2LineError(addr2line::gimli::Error),
    }

    let context = EXECUTABLE_FILE_REQUEST
        .get_response()
        .ok_or(GetLineError::NoExecutableFileResponse)
        .map(|executable_file_response| {
            ElfBytes::<NativeEndian>::minimal_parse({
                let file = executable_file_response.file();
                let ptr = file.addr();
                let len = file.size() as usize;
                unsafe { slice::from_raw_parts(ptr, len) }
            })
        })
        .and_then(|result| result.map_err(|e| GetLineError::ParseElfError(e)))
        .and_then(|elf| {
            Dwarf::load(|section| {
                Ok::<_, ParseError>(EndianSlice::new(
                    {
                        match elf.section_header_by_name(section.name())? {
                            Some(h) => elf.section_data(&h)?.0,
                            None => &[],
                        }
                    },
                    LittleEndian,
                ))
            })
            .map_err(|e| GetLineError::ParseElfError(e))
        })
        .and_then(|mut dwarf| {
            dwarf.file_type = DwarfFileType::Main;
            addr2line::Context::from_dwarf(dwarf).map_err(|e| GetLineError::Addr2LineError(e))
        });
    if let Err(e) = &context {
        log::error!("Error getting function lines: {:?}", e);
    }

    // Print after possible error getting backtrace so the error gets lost instead of actual panic message
    log::error!("{}", info);

    // Safety: We are assuming that the stack is not corrupted
    let call_stack_iterator = unsafe { CallStackIterator::new() };
    for (index, instruction_pointer) in call_stack_iterator.enumerate() {
        let location = context.as_ref().map(|context| {
            context.find_location({
                // Get the previous instruction, which is what we care about
                // https://stackoverflow.com/a/59014431/11145447
                instruction_pointer.get() - 1
            })
        });
        struct DisplayableLocation<'a, E: Debug, E1: Debug> {
            location: Result<Result<Option<Location<'a>>, E>, E1>,
        }
        impl<E: Debug, E1: Debug> Display for DisplayableLocation<'_, E, E1> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match &self.location {
                    Ok(location) => {
                        write!(f, " @ ")?;
                        match location {
                            Ok(name) => match name {
                                Some(location) => {
                                    if let Some(file) = location.file {
                                        write!(f, "{}", file)?;
                                        if let Some(line) = location.line {
                                            write!(f, ":{}", line)?;
                                            if let Some(column) = location.column {
                                                write!(f, ":{}", column)?;
                                            }
                                        }
                                    }
                                }
                                None => write!(f, "<unknown>")?,
                            },
                            Err(e) => write!(f, "<error getting location: {:?}>", e)?,
                        }
                    }
                    Err(_) => {}
                }
                Ok(())
            }
        }

        let displayable_location = DisplayableLocation { location };
        log::error!(
            "  {}: {:#x}{}",
            index,
            instruction_pointer,
            displayable_location
        );
    }

    hlt_loop()
}

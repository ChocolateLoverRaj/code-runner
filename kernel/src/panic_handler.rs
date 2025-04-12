use crate::{
    backtrace_display::{AtLeastLineNumber, BacktraceDisplay, BacktraceEntry, FileInfo},
    call_stack_iterator::CallStackIterator,
    cpu_local_data::get_local,
    hlt_loop::hlt_loop,
    limine_requests::EXECUTABLE_FILE_REQUEST,
    logger_3,
};
use addr2line::{
    fallible_iterator::FallibleIterator,
    gimli::{Dwarf, DwarfFileType, EndianSlice, LittleEndian},
};
use alloc::borrow::ToOwned;
use core::{panic::PanicInfo, slice};
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
    let back_trace = BacktraceDisplay::new({
        // Safety: We are assuming that the stack is not corrupted
        unsafe { CallStackIterator::new() }
            .map(|instruction_pointer| {
                let frame = context.as_ref().ok().and_then(|context| {
                    Some(
                        context
                            .find_frames(instruction_pointer.get() - 1)
                            .skip_all_loads()
                            .ok()?
                            .last()
                            .ok()??,
                    )
                });
                BacktraceEntry {
                    address: instruction_pointer.get(),
                    function_name: frame.as_ref().and_then(|frame| {
                        Some(frame.function.as_ref()?.demangle().ok()?.into_owned())
                    }),
                    file: frame.as_ref().and_then(|frame| {
                        Some({
                            let location = frame.location.as_ref()?;
                            FileInfo {
                                name: location.file?.to_owned(),
                                line_number: location.line.map(|line| AtLeastLineNumber {
                                    line_number: line,
                                    column_number: location.column,
                                }),
                            }
                        })
                    }),
                }
            })
            .collect()
    });
    log::error!("{}\n{}", info, back_trace);

    hlt_loop()
}

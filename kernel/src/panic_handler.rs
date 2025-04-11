use crate::{
    call_stack_iterator::CallStackIterator, cpu_local_data::get_local, hlt_loop::hlt_loop,
    limine_requests::EXECUTABLE_FILE_REQUEST, logger_3,
};
use core::{
    fmt::{Debug, Display},
    panic::PanicInfo,
    slice,
};
use elf::{endian::NativeEndian, ElfBytes};
use rustc_demangle::demangle;
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
        #[error("No symbol table found in ELF")]
        NoSymbolTable,
    }
    let executable_file_response = EXECUTABLE_FILE_REQUEST
        .get_response()
        .ok_or(GetLineError::NoExecutableFileResponse);
    let tables = executable_file_response
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
            elf.symbol_table()
                .map_err(|e| GetLineError::ParseElfError(e))
                .and_then(|option| option.ok_or(GetLineError::NoSymbolTable))
        });
    if let Err(e) = &tables {
        log::error!("Error getting function lines: {:?}", e);
    }

    // Print after possible error getting backtrace so the error gets lost instead of actual panic message
    log::error!("{}", info);

    // Safety: We are assuming that the stack is not corrupted
    let call_stack_iterator = unsafe { CallStackIterator::new() };
    for (index, instruction_pointer) in call_stack_iterator.enumerate() {
        let location = tables.as_ref().ok().map(|(symbol_table, string_table)| {
            symbol_table
                .iter()
                .find(|symbol| {
                    (symbol.st_value..symbol.st_value + symbol.st_size)
                        .contains(&instruction_pointer.into())
                })
                .map(|symbol| string_table.get(symbol.st_name as usize).map(demangle))
        });
        struct DisplayableLocation<T: Display, E: Debug> {
            location: Option<Option<Result<T, E>>>,
        }
        impl<T: Display, E: Debug> Display for DisplayableLocation<T, E> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match &self.location {
                    Some(location) => {
                        write!(f, " @ ")?;
                        match location {
                            Some(name) => match name {
                                Ok(name) => {
                                    write!(f, "{}", name)
                                }
                                Err(e) => {
                                    write!(f, "<error getting name: {:?}>", e)
                                }
                            },
                            None => {
                                write!(f, "<no symbol>")
                            }
                        }
                    }
                    None => Ok(()),
                }
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

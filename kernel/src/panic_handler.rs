use crate::hlt_loop::hlt_loop;
use core::panic::PanicInfo;
use x86_64::instructions::interrupts;

#[cfg(not(test))]
#[panic_handler]
fn kernel_panic_handler(info: &PanicInfo) -> ! {
    // If we don't disable interrupts, code could run while we are in an invalid state. We are in an invalid state from now until reboot because of the panic.

    interrupts::disable();
    log::error!("{}", info);
    hlt_loop()
}

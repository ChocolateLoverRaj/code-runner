use crate::{hlt_loop::hlt_loop, logger_3};
// use crate::init_idt_and_gdt::CPU_LOCAL_APICS;
use core::panic::PanicInfo;
// use x2apic::lapic::IpiAllShorthand;
use x86_64::instructions::interrupts;

// #[cfg(not(test))]
#[panic_handler]
fn kernel_panic_handler(info: &PanicInfo) -> ! {
    // If we don't disable interrupts, code could run while we are in an invalid state. We are in an invalid state from now until reboot because of the panic.

    interrupts::disable();

    // if let Ok(local_apics) = CPU_LOCAL_APICS.try_get() {
    //     if let Ok(local_apic) = local_apics.try_get() {
    //         unsafe { local_apic.force_unlock() };
    //         unsafe {
    //             local_apic
    //                 .lock()
    //                 .send_nmi_all(IpiAllShorthand::AllExcludingSelf)
    //         };
    //     }
    // }
    // // TODO: If the other CPUs have started initializing but did not set the NMI handler yet, we might triple fault. Idk if this is worth fixing though cuz we will only panic if there is a bug in the kernel and the chances of there being a bug that happens right during this timing is very low.

    // Safety: We are already in an undefined state and we just need to simply log the panic information. We will not be logging any more messages.
    unsafe {
        logger_3::force_unlock();
    }
    // To make sure there is a new line before the panic message
    log::error!("Kernel panicked");
    log::error!("{}", info);

    hlt_loop()
}

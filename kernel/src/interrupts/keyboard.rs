// use crate::{apic::LOCAL_APIC, task::keyboard::add_scancode};
// use x86_64::{instructions::port::Port, structures::idt::InterruptStackFrame};

// pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
//     let mut port = Port::new(0x60);
//     let scancode: u8 = unsafe { port.read() };
//     add_scancode(scancode);
//     let mut local_apic = LOCAL_APIC.get().unwrap().lock();
//     unsafe { local_apic.end_of_interrupt() };
// }

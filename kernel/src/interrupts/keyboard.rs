use crate::apic::LOCAL_APIC;
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::{instructions::port::Port, structures::idt::InterruptStackFrame};

static KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(Keyboard::new(
    ScancodeSet1::new(),
    layouts::Us104Key,
    HandleControl::Ignore,
));

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        log::info!("{:?}", key_event);
        // if let Some(key) = keyboard.process_keyevent(key_event) {
        //     log::info!("{:?}", key);
        // }
    }

    let mut local_apic = LOCAL_APIC.get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}

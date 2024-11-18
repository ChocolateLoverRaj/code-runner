use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::{pic::PICS, InterruptIndex};

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        log::info!("Key event: {:?}", key_event);
        // if let Some(key) = keyboard.process_keyevent(key_event) {
        //     match key {
        //         DecodedKey::Unicode(character) => log::info!("{}", character),
        //         DecodedKey::RawKey(key) => log::info!("{:?}", key),
        //     }
        // }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.into());
    }
}

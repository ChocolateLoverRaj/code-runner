use anyhow::anyhow;
use ps2_mouse::{Mouse, MouseState};
use spin::mutex::SpinMutex;
use x86_64::{instructions::port::PortReadOnly, structures::idt::InterruptStackFrame};

use crate::interrupts::{pic::PICS, InterruptIndex};

pub static MOUSE: SpinMutex<Mouse> = SpinMutex::new(Mouse::new());

// An example interrupt based on https://os.phil-opp.com/hardware-interrupts/. The ps2 mouse is configured to fire
// interrupts at PIC offset 12.
pub extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = PortReadOnly::new(0x60);
    let packet = unsafe { port.read() };
    MOUSE.lock().process_packet(packet);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.into());
    }
}
// Initialize the mouse and set the on complete event.
pub fn init() -> anyhow::Result<()> {
    let mut mouse = MOUSE.lock();
    mouse
        .init()
        .map_err(|e| anyhow!("Error initializing PS/2 Mouse: {e:?}"))?;
    mouse.set_on_complete(on_complete);
    Ok(())
}

// This will be fired when a packet is finished being processed.
fn on_complete(mouse_state: MouseState) {
    log::info!("{:?}", mouse_state);
}

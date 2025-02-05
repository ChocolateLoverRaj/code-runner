use x86_64::structures::idt::{self, HandlerFunc};

use super::idt::IdtBuilder;

/// This is not public so it is only used after setting the handler
const SPURIOUS_INTERRUPT_INDEX: u8 = 0xFF;

#[allow(clippy::result_unit_err)]
pub fn set_spurious_interrupt_handler(
    idt_builder: &mut IdtBuilder,
    spurious_interrupt_handler: HandlerFunc,
) -> Result<u8, ()> {
    idt_builder.set_fixed_entry(SPURIOUS_INTERRUPT_INDEX, {
        let mut entry = idt::Entry::missing();
        entry.set_handler_fn(spurious_interrupt_handler);
        entry
    })?;
    Ok(SPURIOUS_INTERRUPT_INDEX)
}

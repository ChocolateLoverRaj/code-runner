// use crate::pic8259_interrupts::Pic8259Interrupts;

// pub struct KeyboardInterruptLock {
//     _guard: spin::MutexGuard<'static, ()>,
// }

// impl KeyboardInterruptLock {
//     pub fn enable(&mut self) {
//         log::debug!("Enabling keyboard interrupts");
//         let mut io_apic = get_io_apic().unwrap().lock();
//         unsafe { io_apic.enable_irq(Pic8259Interrupts::Keyboard.into()) };
//     }

//     pub fn disable(&mut self) {
//         log::debug!("Disabling keyboard interrupts");
//         let mut io_apic = get_io_apic().unwrap().lock();
//         unsafe { io_apic.disable_irq(Pic8259Interrupts::Keyboard.into()) };
//     }
// }

// impl Drop for KeyboardInterruptLock {
//     fn drop(&mut self) {
//         self.disable();
//     }
// }

// pub struct KeyboardInterruptMutex {
//     mutex: spin::Mutex<()>,
// }

// impl KeyboardInterruptMutex {
//     pub fn try_lock(&'static self) -> Option<KeyboardInterruptLock> {
//         self.mutex
//             .try_lock()
//             .map(|mutex_guard| KeyboardInterruptLock {
//                 _guard: mutex_guard,
//             })
//     }
// }

// pub static KEYBOARD_INTERRUPT_MUTEX: KeyboardInterruptMutex = KeyboardInterruptMutex {
//     mutex: spin::Mutex::new(()),
// };

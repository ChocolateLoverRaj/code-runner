use futures_util::StreamExt;
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};

use crate::modules::async_keyboard::AsyncKeyboard;

pub async fn demo_async_keyboard_drop(async_keyboard: &mut AsyncKeyboard) {
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );
    loop {
        let mut count = 0;
        let mut stream = async_keyboard.stream();
        while let Some(scancode) = stream.next().await {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                log::info!("{key_event:?}");
                count += 1;
                if count == 6 {
                    break;
                }
            }
        }
    }
}

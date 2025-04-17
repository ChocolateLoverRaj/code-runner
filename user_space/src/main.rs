#![no_std]
#![no_main]
#![feature(int_roundings)]
#![feature(maybe_uninit_slice)]
extern crate alloc;

pub mod allocator;
pub mod async_keyboard;
pub mod demo_maze_roller_game;
// pub mod draw_rust;
pub mod execute_future;
pub mod executor_context;
pub mod panic_handler;
pub mod screen;
pub mod syscall;

use alloc::format;
use async_keyboard::AsyncKeyboard;
use common::frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics;
use demo_maze_roller_game::demo_maze_roller_game;
use execute_future::execute_future;
use executor_context::ExecutorContext;
use futures::StreamExt;
use pc_keyboard::{layouts::Us104Key, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use screen::Screen;
use syscall::{syscall_exit, syscall_print};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    unsafe { allocator::init() };
    let mut screen = Screen::take().unwrap();
    let mut frame_buffer_embedded_graphics =
        FrameBufferEmbeddedGraphics::new(screen.screen_mut()).unwrap();
    let executor_context = ExecutorContext::default();
    execute_future(
        demo_maze_roller_game(
            &mut frame_buffer_embedded_graphics,
            AsyncKeyboard::init(&executor_context),
        ),
        &executor_context,
    );
    execute_future(
        async {
            let mut async_keyboard = AsyncKeyboard::init(&executor_context);
            syscall_print("Press Ctrl+W to exit this program");
            let mut keyboard = Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore);
            let mut ctrl_pressed = false;
            while let Some(c) = async_keyboard.next().await {
                if let Some(key_event) = keyboard.add_byte(c).unwrap() {
                    match (key_event.code, key_event.state) {
                        (KeyCode::LControl | KeyCode::RControl, key_state) => match key_state {
                            KeyState::Down => {
                                ctrl_pressed = true;
                            }
                            KeyState::Up => {
                                ctrl_pressed = false;
                            }
                            KeyState::SingleShot => {}
                        },
                        (KeyCode::W, KeyState::Down | KeyState::SingleShot) => {
                            if ctrl_pressed {
                                break;
                            }
                        }
                        _ => {}
                    }
                    if let Some(key) = keyboard.process_keyevent(key_event) {
                        syscall_print(&format!("Key pressed: {:?}", key));
                    }
                }
            }
        },
        &executor_context,
    );
    syscall_print("Ctrl+W Pressed. Exiting");
    syscall_exit();
}

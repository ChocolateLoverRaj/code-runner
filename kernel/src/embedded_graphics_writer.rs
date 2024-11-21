use core::fmt::{self, Debug, Write};

use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle},
    prelude::{DrawTarget, Drawable, Point},
    text::{Baseline, Text},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::set_color::SetColor;

struct EmbeddedGraphicsWriterState<D: DrawTarget> {
    draw_target: D,
    x: u32,
    y: u32,
}

pub struct EmbeddedGraphicsWriter<D: DrawTarget> {
    state: spin::Mutex<EmbeddedGraphicsWriterState<D>>,
    color: D::Color,
    font: MonoFont<'static>,
    background_color: D::Color,
}

impl<D: DrawTarget> EmbeddedGraphicsWriter<D>
where
    D::Color: Default,
{
    pub fn new(draw_target: D, font: MonoFont<'static>, background_color: D::Color) -> Self {
        Self {
            state: spin::Mutex::new(EmbeddedGraphicsWriterState {
                draw_target,
                x: 0,
                y: 0,
            }),
            color: D::Color::default(),
            font,
            background_color,
        }
    }
}

impl<D: DrawTarget + Send> Write for EmbeddedGraphicsWriter<D>
where
    D::Color: Send + Sync,
    D::Error: Debug,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut state = self.state.lock();
        if state.x == 0 && state.y == 0 {
            state
                .draw_target
                .clear(self.background_color)
                .map_err(|_e| fmt::Error)?;
        }
        let inc_y = |state: &mut EmbeddedGraphicsWriterState<D>| {
            state.y += 1;

            if state.y
                == state.draw_target.bounding_box().size.height / self.font.character_size.height
            {
                // TODO: Shift all previous lines up 1 line and keep writing to the final line
                state.y = 0;
            }
        };
        for character in s.graphemes(true) {
            match character {
                "\n" | "\r\n" => {
                    state.x = 0;
                    inc_y(&mut state);
                }
                character => {
                    let top_y = state.y * self.font.character_size.height;
                    let left_x = state.x * self.font.character_size.width;
                    Text::with_baseline(
                        character,
                        Point::new(left_x as i32, top_y as i32),
                        MonoTextStyle::new(&self.font, self.color),
                        Baseline::Top,
                    )
                    .draw(&mut state.draw_target)
                    .map_err(|_e| core::fmt::Error)?;
                    state.x += 1;
                    if state.x
                        == state.draw_target.bounding_box().size.width
                            / self.font.character_size.width
                    {
                        state.x = 0;
                        inc_y(&mut state);
                    }
                }
            }
        }
        Ok(())
    }
}

impl<D: DrawTarget> SetColor<D::Color> for EmbeddedGraphicsWriter<D> {
    fn set_color(&mut self, color: D::Color) {
        self.color = color;
    }
}

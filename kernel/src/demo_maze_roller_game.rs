use bootloader_api::info::FrameBuffer;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, RgbColor, Size, WebColors},
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, StyledDrawable},
};
use futures_util::StreamExt;
use pc_keyboard::{layouts, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};

use crate::{
    frame_buffer::{Display, Position},
    modules::async_keyboard::AsyncKeyboard,
};

#[derive(Debug, PartialEq, Eq)]
enum Cell {
    Air,
    Wall,
    Start,
    End,
}

#[derive(Debug)]
enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

trait PositionMove {
    /// Returns false if moving would make a x or y out of bounds
    fn try_move(&mut self, direction: MoveDirection) -> bool;
}

impl PositionMove for Position {
    fn try_move(&mut self, direction: MoveDirection) -> bool {
        match direction {
            MoveDirection::Up => match self.y.checked_sub(1) {
                Some(new_y) => {
                    self.y = new_y;
                    true
                }
                None => false,
            },
            MoveDirection::Down => match self.y.checked_add(1) {
                Some(new_y) => {
                    self.y = new_y;
                    true
                }
                None => false,
            },
            MoveDirection::Left => match self.x.checked_sub(1) {
                Some(new_x) => {
                    self.x = new_x;
                    true
                }
                None => false,
            },
            MoveDirection::Right => match self.x.checked_add(1) {
                Some(new_x) => {
                    self.x = new_x;
                    true
                }
                None => false,
            },
        }
    }
}

impl Cell {
    pub fn get_color(&self) -> Rgb888 {
        match self {
            Self::Air => Rgb888::BLACK,
            Self::Wall => Rgb888::GREEN,
            Self::Start => Rgb888::MAGENTA,
            Self::End => Rgb888::CSS_PINK,
        }
    }

    pub fn can_enter(&self) -> bool {
        match self {
            Cell::Wall => false,
            _ => true,
        }
    }
}

const LEVELS: &[&[&[Cell]]] = &[&[
    &[Cell::Wall, Cell::Air, Cell::Air, Cell::End],
    &[Cell::Wall, Cell::Air, Cell::Wall, Cell::Wall],
    &[Cell::Start, Cell::Air, Cell::Air, Cell::Wall],
    &[Cell::Wall, Cell::Wall, Cell::Wall, Cell::Wall],
]];

pub async fn demo_maze_roller_game(
    frame_buffer: &mut FrameBuffer,
    async_keyboard: &mut AsyncKeyboard,
) {
    let mut display = Display::new(frame_buffer);
    display.clear(Rgb888::CSS_GRAY).unwrap();
    let mut current_level = 0;
    let mut stream = async_keyboard
        .stream()
        .filter_map({
            let mut keyboard = Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore,
            );
            move |scancode| {
                let output = keyboard.add_byte(scancode).ok().flatten();
                async { output }
            }
        })
        .filter(|key_event| {
            let output = match key_event.state {
                KeyState::Down | KeyState::SingleShot => true,
                KeyState::Up => false,
            };
            async move { output }
        })
        .boxed();
    loop {
        let level = LEVELS[current_level];
        let height = level.len();
        let width = level[0].len();
        let initial_position = level
            .iter()
            .enumerate()
            .find_map(|(y, row)| {
                row.iter()
                    .position(|cell| cell == &Cell::Start)
                    .map(|x| Position { x, y })
            })
            .unwrap();
        let mut current_position = initial_position;
        let cell_size = {
            let max_cell_height = display.bounding_box().size.height.div_floor(height as u32);
            let max_cell_width = display.bounding_box().size.width.div_floor(width as u32);
            max_cell_height.min(max_cell_width)
        };
        let get_point = |position: Position| {
            let Position { x, y } = position;
            Point::new((x as u32 * cell_size) as i32, (y as u32 * cell_size) as i32)
        };
        let get_cell = |Position { x, y }: Position| -> &Cell { &level[y][x] };
        let try_get_cell = |Position { x, y }: Position| -> Option<&Cell> {
            level.get(y).and_then(|row| row.get(x))
        };
        // Draw level
        loop {
            for y in 0..height {
                for x in 0..width {
                    let draw_position = Position { x, y };
                    let cell = get_cell(draw_position);
                    Rectangle::new(get_point(draw_position), Size::new(cell_size, cell_size))
                        .draw_styled(
                            &PrimitiveStyleBuilder::new()
                                .fill_color(cell.get_color())
                                .build(),
                            &mut display,
                        )
                        .unwrap();
                }
            }
            // Draw ball
            Circle::new(get_point(current_position), cell_size)
                .draw_styled(
                    &PrimitiveStyleBuilder::new()
                        .fill_color(Rgb888::BLUE)
                        .build(),
                    &mut display,
                )
                .unwrap();

            loop {
                #[derive(Debug)]
                enum Input {
                    Move(MoveDirection),
                    ResetLevel,
                }
                let input = loop {
                    match stream.next().await.unwrap().code {
                        KeyCode::ArrowUp => break Input::Move(MoveDirection::Up),
                        KeyCode::ArrowDown => break Input::Move(MoveDirection::Down),
                        KeyCode::ArrowLeft => break Input::Move(MoveDirection::Left),
                        KeyCode::ArrowRight => break Input::Move(MoveDirection::Right),
                        KeyCode::R => break Input::ResetLevel,
                        _ => {}
                    }
                };
                match input {
                    Input::Move(move_direction) => {
                        log::info!("Move to: {move_direction:?}");
                        let mut attempted_position_to_move_to = current_position.clone();
                        let new_position_valid =
                            attempted_position_to_move_to.try_move(move_direction);
                        if new_position_valid
                            && try_get_cell(attempted_position_to_move_to)
                                .filter(|cell| cell.can_enter())
                                .is_some()
                        {
                            current_position = attempted_position_to_move_to;
                            if get_cell(current_position) == &Cell::End {
                                log::info!("LEVEL COMPLETE");
                            }
                            break;
                        }
                    }
                    Input::ResetLevel => {
                        current_position = initial_position;
                        break;
                    }
                }
            }
        }
    }
}

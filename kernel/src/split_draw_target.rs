// use core::convert::Infallible;

// use embedded_graphics::{
//     prelude::{Dimensions, DrawTarget, OriginDimensions, PixelColor, Point, Size},
//     primitives::Rectangle,
// };

// pub struct SplitDrawTarget<'a, SuperTarget: DrawTarget> {
//     draw_target: &'a SuperTarget,
//     bounding_box: Rectangle,
// }

// impl<'a, D: DrawTarget> OriginDimensions for SplitDrawTarget<'a, D> {
//     fn size(&self) -> embedded_graphics::prelude::Size {
//         self.bounding_box.size
//     }
// }

// impl<'a, D: DrawTarget> DrawTarget for SplitDrawTarget<'a, D> {
//     type Color = D::Color;

//     type Error = D::Error;

//     fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
//     where
//         I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
//     {
//         self.draw_target.draw_iter(pixels)
//     }
// }

// pub fn split_draw_target<'a, D: DrawTarget>(
//     draw_target: &'a D,
//     x: i32,
// ) -> (SplitDrawTarget<'a, D>, SplitDrawTarget<'a, D>) {
//     (
//         SplitDrawTarget {
//             draw_target,
//             bounding_box: Rectangle {
//                 top_left: Point { x: 0, y: 0 },
//                 size: Size::new(x as u32, draw_target.bounding_box().size.height),
//             },
//         },
//         SplitDrawTarget {
//             draw_target,
//             bounding_box: Rectangle {
//                 top_left: Point { x, y: 0 },
//                 size: Size::new(
//                     draw_target.bounding_box().size.width - x as u32,
//                     draw_target.bounding_box().size.height,
//                 ),
//             },
//         },
//     )
// }

use lgfx::{self, ColorRgb332, Gfx, LgfxGuard};
use lgfx::{DrawImage, DrawPrimitives};
use crate::lgfx::{EpdMode, DrawChars, FontManupulation, LgfxDisplay};
use anyhow::Result;

pub struct Chart {
    width: i32,
    height: i32,
    foreground: ColorRgb332,
    background: ColorRgb332,
}

impl Chart {
    pub fn new(width: i32, height: i32, foreground: ColorRgb332, background: ColorRgb332) -> Self {
        Self {
            width,
            height,
            foreground,
            background
        }
    }

    pub fn draw<D: DrawPrimitives<ColorRgb332>, F: FnMut(usize) -> Option<f32>>(&self, target: &mut D, left: i32, top: i32, item_count: usize, min_value: f32, max_value: f32, mut values: F) -> anyhow::Result<()> {
        if item_count == 0 {
            return Ok(());
        }

        let width = self.width;
        let height = self.height;
        let right = left + width - 1;
        let bottom = top + height - 1;
        let foreground = self.foreground;
        //let background = self.background;
        target.draw_line(left, top, right, top, foreground);
        target.draw_line(left, bottom, right, bottom, foreground);
        target.draw_line(left, top, left, bottom, foreground);
        target.draw_line(right, top, right, bottom, foreground);
        
        let range = max_value - min_value;
        if range == 0.0 {
            return Ok(());
        }
        let mut prev_point: Option<(i32, i32)> = None;

        for index in 0..item_count {
            let x = (width * (index as i32) + (item_count as i32) / 2 - 1) / (item_count as i32) + left;
            if let Some(value) = values(index) {
                let y = bottom - ((value - min_value) * (height as f32) / range).round() as i32;
                if let Some((prev_x, prev_y)) = prev_point {
                    target.draw_line(prev_x, prev_y, x, y, foreground);
                }
                prev_point = Some((x, y));
            } else {
                prev_point = None;
            }
        }
        
        Ok(())
    }
}
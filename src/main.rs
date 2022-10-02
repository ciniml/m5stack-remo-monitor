use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
mod lgfx;

const LOGO_PNG: &[u8; 9278] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/rust-logo-512x512-blk_white.png"
));

use lgfx::{DrawImage, DrawPrimitives, Gfx};

use crate::lgfx::{DrawChars, FontManupulation};

fn main() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    println!("Hello, world!");
    let gfx = Gfx::setup().unwrap();
    gfx.fill_rect(0, 0, 32, 32, lgfx::ColorRgb332::new(0));
    gfx.draw_png(LOGO_PNG)
        .postion(32, 0)
        .scale(0.8, 0.0)
        .execute();
    gfx.set_font(lgfx::LgfxFontId::Font4).unwrap();
    gfx.set_text_size(2.0, 2.0);
    gfx.draw_chars(
        "Hello, Rust!",
        0,
        640,
        lgfx::ColorRgb332::new(0),
        lgfx::ColorRgb332::new(0xff),
        1.0,
        1.0,
    );
    gfx.draw_line(100, 600, 200, 700, lgfx::ColorRgb332::new(0));
    let sprite = gfx.create_sprite(64, 64).unwrap();
    sprite.clear(lgfx::ColorRgb332::new(0xff));
    sprite.fill_rect(0, 0, 32, 32, lgfx::ColorRgb332::new(0));
    sprite.fill_rect(32, 32, 32, 32, lgfx::ColorRgb332::new(0));
    sprite.push_sprite(0, 512);
    sprite.push_sprite(512 - 64, 512);
}

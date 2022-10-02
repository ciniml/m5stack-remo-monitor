
use esp_idf_sys::lgfx_sys::*;
use num_enum::IntoPrimitive;

pub struct Gfx {
    target: lgfx_target_t,
}

static mut GFX_INITIALIZED: bool = false;
impl Gfx {
    pub fn setup() -> Option<Gfx> {
        if unsafe { GFX_INITIALIZED } {
            None
        } else {
            unsafe {
                GFX_INITIALIZED = true;
            }
            Some(Gfx {
                target: unsafe { lgfx_c_setup() },
            })
        }
    }
    pub fn create_sprite(&self, w: i32, h: i32) -> Result<Sprite, ()> {
        Sprite::new(self, w, h)
    }
}
impl LgfxTarget for Gfx {
    fn target(&self) -> lgfx_target_t {
        self.target
    }
}

impl<Target> DrawImage for Target
where
    Target: LgfxTarget,
{
    fn draw_png<'a>(&self, data: &'a [u8]) -> DrawPng<'a> {
        DrawPng::new(self.target(), data)
    }
}

pub struct Sprite {
    target: lgfx_target_t,
}
impl Sprite {
    fn new(gfx: &Gfx, w: i32, h: i32) -> Result<Self, ()> {
        let sprite = unsafe { lgfx_c_create_sprite(gfx.target(), w, h) };
        if sprite == core::ptr::null_mut() {
            Err(())
        } else {
            Ok(Self { target: sprite })
        }
    }
    pub fn push_sprite(&self, x: i32, y: i32) {
        unsafe { lgfx_c_push_sprite(self.target, x, y) };
    }
}
impl LgfxTarget for Sprite {
    fn target(&self) -> lgfx_target_t {
        self.target
    }
}
impl Drop for Sprite {
    fn drop(&mut self) {
        unsafe { lgfx_c_delete_sprite(self.target) };
    }
}

trait LgfxTarget {
    fn target(&self) -> lgfx_target_t;
}

pub trait DrawImage {
    fn draw_png<'a>(&self, data: &'a [u8]) -> DrawPng<'a>;
}

pub trait Color: Clone {
    fn as_u32(&self) -> u32;
}

#[derive(Debug, Clone)]
pub struct ColorRgb332 {
    raw: u8,
}
impl ColorRgb332 {
    pub fn new(raw: u8) -> Self {
        Self { raw }
    }
}
impl Color for ColorRgb332 {
    fn as_u32(&self) -> u32 {
        let r = (self.raw & 0xe0) << 0;
        let g = (self.raw & 0x1c) << 3;
        let b = (self.raw & 0x03) << 6;
        (((r | ((0u8.wrapping_sub((r >> 5) & 1)) & 0x1f)) as u32) << 16)
            | (((g | ((0u8.wrapping_sub((g >> 5) & 1)) & 0x1f)) as u32) << 8)
            | ((b | ((0u8.wrapping_sub((b >> 6) & 1)) & 0x3f)) as u32)
    }
}

#[derive(Debug, Clone)]
pub struct ColorRgb888 {
    raw: u32,
}
impl ColorRgb888 {
    pub fn new(raw: u32) -> Self {
        Self { raw }
    }
}
impl Color for ColorRgb888 {
    fn as_u32(&self) -> u32 {
        self.raw & 0xffffff
    }
}

pub trait DrawPrimitives<C: Color> {
    fn clear(&self, color: C);
    fn fill_rect(&self, x: i32, y: i32, w: i32, h: i32, color: C);
    fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32, color: C);
}

impl<Target> DrawPrimitives<ColorRgb332> for Target
where
    Target: LgfxTarget,
{
    fn clear(&self, color: ColorRgb332) {
        unsafe {
            lgfx_c_clear_rgb332(self.target(), color.raw);
        }
    }
    fn fill_rect(&self, x: i32, y: i32, w: i32, h: i32, color: ColorRgb332) {
        unsafe {
            lgfx_c_fill_rect_rgb332(self.target(), x, y, w, h, color.raw);
        }
    }
    fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32, color: ColorRgb332) {
        unsafe {
            lgfx_c_draw_line_rgb332(self.target(), x0, y0, x1, y1, color.raw);
        }
    }
}
impl<Target> DrawPrimitives<ColorRgb888> for Target
where
    Target: LgfxTarget,
{
    fn clear(&self, color: ColorRgb888) {
        unsafe {
            lgfx_c_clear_rgb888(self.target(), color.raw);
        }
    }
    fn fill_rect(&self, x: i32, y: i32, w: i32, h: i32, color: ColorRgb888) {
        unsafe {
            lgfx_c_fill_rect_rgb888(self.target(), x, y, w, h, color.raw);
        }
    }
    fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32, color: ColorRgb888) {
        unsafe {
            lgfx_c_draw_line_rgb888(self.target(), x0, y0, x1, y1, color.raw);
        }
    }
}

pub trait DrawChar<C: Color> {
    fn draw_char(&self, c: char, x: i32, y: i32, fg: C, bg: C, size_x: f32, size_y: f32) -> i32;
}
pub trait DrawChars<C: Color> {
    fn draw_chars(&self, s: &str, x: i32, y: i32, fg: C, bg: C, size_x: f32, size_y: f32) -> i32;
}

impl<Target> DrawChar<ColorRgb332> for Target
where
    Target: LgfxTarget,
{
    fn draw_char(
        &self,
        c: char,
        x: i32,
        y: i32,
        fg: ColorRgb332,
        bg: ColorRgb332,
        size_x: f32,
        size_y: f32,
    ) -> i32 {
        let mut buf = [0u16; 2];
        let encoded = c.encode_utf16(&mut buf);
        let mut width = 0;

        width += if encoded.len() >= 1 {
            unsafe {
                lgfx_c_draw_char_rgb332(
                    self.target(),
                    x,
                    y,
                    encoded[0],
                    fg.raw,
                    bg.raw,
                    size_x,
                    size_y,
                ) as i32
            }
        } else {
            0
        };
        width += if encoded.len() >= 2 {
            unsafe {
                lgfx_c_draw_char_rgb332(
                    self.target(),
                    x,
                    y,
                    encoded[1],
                    fg.raw,
                    bg.raw,
                    size_x,
                    size_y,
                ) as i32
            }
        } else {
            0
        };
        width
    }
}
impl<Target> DrawChar<ColorRgb888> for Target
where
    Target: LgfxTarget,
{
    fn draw_char(
        &self,
        c: char,
        x: i32,
        y: i32,
        fg: ColorRgb888,
        bg: ColorRgb888,
        size_x: f32,
        size_y: f32,
    ) -> i32 {
        let mut buf = [0u16; 2];
        let encoded = c.encode_utf16(&mut buf);
        let mut width = 0;

        width += if encoded.len() >= 1 {
            unsafe {
                lgfx_c_draw_char_rgb888(
                    self.target(),
                    x,
                    y,
                    encoded[0],
                    fg.raw,
                    bg.raw,
                    size_x,
                    size_y,
                ) as i32
            }
        } else {
            0
        };
        width += if encoded.len() >= 2 {
            unsafe {
                lgfx_c_draw_char_rgb888(
                    self.target(),
                    x,
                    y,
                    encoded[1],
                    fg.raw,
                    bg.raw,
                    size_x,
                    size_y,
                ) as i32
            }
        } else {
            0
        };
        width
    }
}
impl<Target, C> DrawChars<C> for Target
where
    Target: LgfxTarget + DrawChar<C>,
    C: Color,
{
    fn draw_chars(&self, s: &str, x: i32, y: i32, fg: C, bg: C, size_x: f32, size_y: f32) -> i32 {
        let mut width = 0;
        for c in s.chars() {
            width += self.draw_char(c, x + width, y, fg.clone(), bg.clone(), size_x, size_y);
        }
        width
    }
}

pub trait FontManupulation {
    fn set_font(&self, font: LgfxFontId) -> Result<(), ()>;
    fn set_text_size(&self, sx: f32, sy: f32);
}
impl<Target: LgfxTarget> FontManupulation for Target {
    fn set_font(&self, font: LgfxFontId) -> Result<(), ()> {
        let success = unsafe { lgfx_c_set_font(self.target(), font.into()) };
        if success {
            Ok(())
        } else {
            Err(())
        }
    }
    fn set_text_size(&self, sx: f32, sy: f32) {
        unsafe {
            lgfx_c_set_text_size(self.target(), sx, sy);
        }
    }
}

#[must_use]
pub struct DrawPng<'a> {
    target: lgfx_target_t,
    data: &'a [u8],
    x: i32,
    y: i32,
    max_width: i32,
    max_height: i32,
    offset_x: i32,
    offset_y: i32,
    scale_x: f32,
    scale_y: f32,
    datum_: textdatum_t,
}

impl<'a> DrawPng<'a> {
    const fn new(target: lgfx_target_t, data: &'a [u8]) -> Self {
        Self {
            target,
            data,
            x: 0,
            y: 0,
            max_width: 0,
            max_height: 0,
            offset_x: 0,
            offset_y: 0,
            scale_x: 1.0,
            scale_y: 0.0,
            datum_: textdatum_t_top_left,
        }
    }
    pub fn postion(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn max_size(mut self, max_width: i32, max_height: i32) -> Self {
        self.max_width = max_width;
        self.max_height = max_height;
        self
    }
    pub fn offset(mut self, offset_x: i32, offset_y: i32) -> Self {
        self.offset_x = offset_x;
        self.offset_y = offset_y;
        self
    }
    pub fn scale(mut self, scale_x: f32, scale_y: f32) -> Self {
        self.scale_x = scale_x;
        self.scale_y = scale_y;
        self
    }
    pub fn datum(mut self, datum: textdatum_t) -> Self {
        self.datum_ = datum;
        self
    }
    pub fn execute(self) {
        unsafe {
            lgfx_c_draw_png(
                self.target,
                self.data.as_ptr(),
                self.data.len() as u32,
                self.x,
                self.y,
                self.max_width,
                self.max_height,
                self.offset_x,
                self.offset_y,
                self.scale_x,
                self.scale_y,
                self.datum_,
            )
        };
    }
}

#[allow(non_camel_case_types)]
#[derive(IntoPrimitive)]
#[repr(u32)]
pub enum LgfxFontId {
    Font0 = lgfx_font_id_t_Font0,
    Font2 = lgfx_font_id_t_Font2,
    Font4 = lgfx_font_id_t_Font4,
    Font6 = lgfx_font_id_t_Font6,
    Font7 = lgfx_font_id_t_Font7,
    Font8 = lgfx_font_id_t_Font8,
    Font8x8C64 = lgfx_font_id_t_Font8x8C64,
    AsciiFont8x16 = lgfx_font_id_t_AsciiFont8x16,
    AsciiFont24x48 = lgfx_font_id_t_AsciiFont24x48,
    TomThumb = lgfx_font_id_t_TomThumb,
    FreeMono9pt7b = lgfx_font_id_t_FreeMono9pt7b,
    FreeMono12pt7b = lgfx_font_id_t_FreeMono12pt7b,
    FreeMono18pt7b = lgfx_font_id_t_FreeMono18pt7b,
    FreeMono24pt7b = lgfx_font_id_t_FreeMono24pt7b,
    FreeMonoBold9pt7b = lgfx_font_id_t_FreeMonoBold9pt7b,
    FreeMonoBold12pt7b = lgfx_font_id_t_FreeMonoBold12pt7b,
    FreeMonoBold18pt7b = lgfx_font_id_t_FreeMonoBold18pt7b,
    FreeMonoBold24pt7b = lgfx_font_id_t_FreeMonoBold24pt7b,
    FreeMonoOblique9pt7b = lgfx_font_id_t_FreeMonoOblique9pt7b,
    FreeMonoOblique12pt7b = lgfx_font_id_t_FreeMonoOblique12pt7b,
    FreeMonoOblique18pt7b = lgfx_font_id_t_FreeMonoOblique18pt7b,
    FreeMonoOblique24pt7b = lgfx_font_id_t_FreeMonoOblique24pt7b,
    FreeMonoBoldOblique9pt7b = lgfx_font_id_t_FreeMonoBoldOblique9pt7b,
    FreeMonoBoldOblique12pt7b = lgfx_font_id_t_FreeMonoBoldOblique12pt7b,
    FreeMonoBoldOblique18pt7b = lgfx_font_id_t_FreeMonoBoldOblique18pt7b,
    FreeMonoBoldOblique24pt7b = lgfx_font_id_t_FreeMonoBoldOblique24pt7b,
    FreeSans9pt7b = lgfx_font_id_t_FreeSans9pt7b,
    FreeSans12pt7b = lgfx_font_id_t_FreeSans12pt7b,
    FreeSans18pt7b = lgfx_font_id_t_FreeSans18pt7b,
    FreeSans24pt7b = lgfx_font_id_t_FreeSans24pt7b,
    FreeSansBold9pt7b = lgfx_font_id_t_FreeSansBold9pt7b,
    FreeSansBold12pt7b = lgfx_font_id_t_FreeSansBold12pt7b,
    FreeSansBold18pt7b = lgfx_font_id_t_FreeSansBold18pt7b,
    FreeSansBold24pt7b = lgfx_font_id_t_FreeSansBold24pt7b,
    FreeSansOblique9pt7b = lgfx_font_id_t_FreeSansOblique9pt7b,
    FreeSansOblique12pt7b = lgfx_font_id_t_FreeSansOblique12pt7b,
    FreeSansOblique18pt7b = lgfx_font_id_t_FreeSansOblique18pt7b,
    FreeSansOblique24pt7b = lgfx_font_id_t_FreeSansOblique24pt7b,
    FreeSansBoldOblique9pt7b = lgfx_font_id_t_FreeSansBoldOblique9pt7b,
    FreeSansBoldOblique12pt7b = lgfx_font_id_t_FreeSansBoldOblique12pt7b,
    FreeSansBoldOblique18pt7b = lgfx_font_id_t_FreeSansBoldOblique18pt7b,
    FreeSansBoldOblique24pt7b = lgfx_font_id_t_FreeSansBoldOblique24pt7b,
    FreeSerif9pt7b = lgfx_font_id_t_FreeSerif9pt7b,
    FreeSerif12pt7b = lgfx_font_id_t_FreeSerif12pt7b,
    FreeSerif18pt7b = lgfx_font_id_t_FreeSerif18pt7b,
    FreeSerif24pt7b = lgfx_font_id_t_FreeSerif24pt7b,
    FreeSerifItalic9pt7b = lgfx_font_id_t_FreeSerifItalic9pt7b,
    FreeSerifItalic12pt7b = lgfx_font_id_t_FreeSerifItalic12pt7b,
    FreeSerifItalic18pt7b = lgfx_font_id_t_FreeSerifItalic18pt7b,
    FreeSerifItalic24pt7b = lgfx_font_id_t_FreeSerifItalic24pt7b,
    FreeSerifBold9pt7b = lgfx_font_id_t_FreeSerifBold9pt7b,
    FreeSerifBold12pt7b = lgfx_font_id_t_FreeSerifBold12pt7b,
    FreeSerifBold18pt7b = lgfx_font_id_t_FreeSerifBold18pt7b,
    FreeSerifBold24pt7b = lgfx_font_id_t_FreeSerifBold24pt7b,
    FreeSerifBoldItalic9pt7b = lgfx_font_id_t_FreeSerifBoldItalic9pt7b,
    FreeSerifBoldItalic12pt7b = lgfx_font_id_t_FreeSerifBoldItalic12pt7b,
    FreeSerifBoldItalic18pt7b = lgfx_font_id_t_FreeSerifBoldItalic18pt7b,
    FreeSerifBoldItalic24pt7b = lgfx_font_id_t_FreeSerifBoldItalic24pt7b,
    Orbitron_Light_24 = lgfx_font_id_t_Orbitron_Light_24,
    Orbitron_Light_32 = lgfx_font_id_t_Orbitron_Light_32,
    Roboto_Thin_24 = lgfx_font_id_t_Roboto_Thin_24,
    Satisfy_24 = lgfx_font_id_t_Satisfy_24,
    Yellowtail_32 = lgfx_font_id_t_Yellowtail_32,
    DejaVu9 = lgfx_font_id_t_DejaVu9,
    DejaVu12 = lgfx_font_id_t_DejaVu12,
    DejaVu18 = lgfx_font_id_t_DejaVu18,
    DejaVu24 = lgfx_font_id_t_DejaVu24,
    DejaVu40 = lgfx_font_id_t_DejaVu40,
    DejaVu56 = lgfx_font_id_t_DejaVu56,
    DejaVu72 = lgfx_font_id_t_DejaVu72,
    lgfxJapanMincho_8 = lgfx_font_id_t_lgfxJapanMincho_8,
    lgfxJapanMincho_12 = lgfx_font_id_t_lgfxJapanMincho_12,
    lgfxJapanMincho_16 = lgfx_font_id_t_lgfxJapanMincho_16,
    lgfxJapanMincho_20 = lgfx_font_id_t_lgfxJapanMincho_20,
    lgfxJapanMincho_24 = lgfx_font_id_t_lgfxJapanMincho_24,
    lgfxJapanMincho_28 = lgfx_font_id_t_lgfxJapanMincho_28,
    lgfxJapanMincho_32 = lgfx_font_id_t_lgfxJapanMincho_32,
    lgfxJapanMincho_36 = lgfx_font_id_t_lgfxJapanMincho_36,
    lgfxJapanMincho_40 = lgfx_font_id_t_lgfxJapanMincho_40,
    lgfxJapanMinchoP_8 = lgfx_font_id_t_lgfxJapanMinchoP_8,
    lgfxJapanMinchoP_12 = lgfx_font_id_t_lgfxJapanMinchoP_12,
    lgfxJapanMinchoP_16 = lgfx_font_id_t_lgfxJapanMinchoP_16,
    lgfxJapanMinchoP_20 = lgfx_font_id_t_lgfxJapanMinchoP_20,
    lgfxJapanMinchoP_24 = lgfx_font_id_t_lgfxJapanMinchoP_24,
    lgfxJapanMinchoP_28 = lgfx_font_id_t_lgfxJapanMinchoP_28,
    lgfxJapanMinchoP_32 = lgfx_font_id_t_lgfxJapanMinchoP_32,
    lgfxJapanMinchoP_36 = lgfx_font_id_t_lgfxJapanMinchoP_36,
    lgfxJapanMinchoP_40 = lgfx_font_id_t_lgfxJapanMinchoP_40,
    lgfxJapanGothic_8 = lgfx_font_id_t_lgfxJapanGothic_8,
    lgfxJapanGothic_12 = lgfx_font_id_t_lgfxJapanGothic_12,
    lgfxJapanGothic_16 = lgfx_font_id_t_lgfxJapanGothic_16,
    lgfxJapanGothic_20 = lgfx_font_id_t_lgfxJapanGothic_20,
    lgfxJapanGothic_24 = lgfx_font_id_t_lgfxJapanGothic_24,
    lgfxJapanGothic_28 = lgfx_font_id_t_lgfxJapanGothic_28,
    lgfxJapanGothic_32 = lgfx_font_id_t_lgfxJapanGothic_32,
    lgfxJapanGothic_36 = lgfx_font_id_t_lgfxJapanGothic_36,
    lgfxJapanGothic_40 = lgfx_font_id_t_lgfxJapanGothic_40,
    lgfxJapanGothicP_8 = lgfx_font_id_t_lgfxJapanGothicP_8,
    lgfxJapanGothicP_12 = lgfx_font_id_t_lgfxJapanGothicP_12,
    lgfxJapanGothicP_16 = lgfx_font_id_t_lgfxJapanGothicP_16,
    lgfxJapanGothicP_20 = lgfx_font_id_t_lgfxJapanGothicP_20,
    lgfxJapanGothicP_24 = lgfx_font_id_t_lgfxJapanGothicP_24,
    lgfxJapanGothicP_28 = lgfx_font_id_t_lgfxJapanGothicP_28,
    lgfxJapanGothicP_32 = lgfx_font_id_t_lgfxJapanGothicP_32,
    lgfxJapanGothicP_36 = lgfx_font_id_t_lgfxJapanGothicP_36,
    lgfxJapanGothicP_40 = lgfx_font_id_t_lgfxJapanGothicP_40,
    efontCN_10 = lgfx_font_id_t_efontCN_10,
    efontCN_10_b = lgfx_font_id_t_efontCN_10_b,
    efontCN_10_bi = lgfx_font_id_t_efontCN_10_bi,
    efontCN_10_i = lgfx_font_id_t_efontCN_10_i,
    efontCN_12 = lgfx_font_id_t_efontCN_12,
    efontCN_12_b = lgfx_font_id_t_efontCN_12_b,
    efontCN_12_bi = lgfx_font_id_t_efontCN_12_bi,
    efontCN_12_i = lgfx_font_id_t_efontCN_12_i,
    efontCN_14 = lgfx_font_id_t_efontCN_14,
    efontCN_14_b = lgfx_font_id_t_efontCN_14_b,
    efontCN_14_bi = lgfx_font_id_t_efontCN_14_bi,
    efontCN_14_i = lgfx_font_id_t_efontCN_14_i,
    efontCN_16 = lgfx_font_id_t_efontCN_16,
    efontCN_16_b = lgfx_font_id_t_efontCN_16_b,
    efontCN_16_bi = lgfx_font_id_t_efontCN_16_bi,
    efontCN_16_i = lgfx_font_id_t_efontCN_16_i,
    efontCN_24 = lgfx_font_id_t_efontCN_24,
    efontCN_24_b = lgfx_font_id_t_efontCN_24_b,
    efontCN_24_bi = lgfx_font_id_t_efontCN_24_bi,
    efontCN_24_i = lgfx_font_id_t_efontCN_24_i,
    efontJA_10 = lgfx_font_id_t_efontJA_10,
    efontJA_10_b = lgfx_font_id_t_efontJA_10_b,
    efontJA_10_bi = lgfx_font_id_t_efontJA_10_bi,
    efontJA_10_i = lgfx_font_id_t_efontJA_10_i,
    efontJA_12 = lgfx_font_id_t_efontJA_12,
    efontJA_12_b = lgfx_font_id_t_efontJA_12_b,
    efontJA_12_bi = lgfx_font_id_t_efontJA_12_bi,
    efontJA_12_i = lgfx_font_id_t_efontJA_12_i,
    efontJA_14 = lgfx_font_id_t_efontJA_14,
    efontJA_14_b = lgfx_font_id_t_efontJA_14_b,
    efontJA_14_bi = lgfx_font_id_t_efontJA_14_bi,
    efontJA_14_i = lgfx_font_id_t_efontJA_14_i,
    efontJA_16 = lgfx_font_id_t_efontJA_16,
    efontJA_16_b = lgfx_font_id_t_efontJA_16_b,
    efontJA_16_bi = lgfx_font_id_t_efontJA_16_bi,
    efontJA_16_i = lgfx_font_id_t_efontJA_16_i,
    efontJA_24 = lgfx_font_id_t_efontJA_24,
    efontJA_24_b = lgfx_font_id_t_efontJA_24_b,
    efontJA_24_bi = lgfx_font_id_t_efontJA_24_bi,
    efontJA_24_i = lgfx_font_id_t_efontJA_24_i,
    efontKR_10 = lgfx_font_id_t_efontKR_10,
    efontKR_10_b = lgfx_font_id_t_efontKR_10_b,
    efontKR_10_bi = lgfx_font_id_t_efontKR_10_bi,
    efontKR_10_i = lgfx_font_id_t_efontKR_10_i,
    efontKR_12 = lgfx_font_id_t_efontKR_12,
    efontKR_12_b = lgfx_font_id_t_efontKR_12_b,
    efontKR_12_bi = lgfx_font_id_t_efontKR_12_bi,
    efontKR_12_i = lgfx_font_id_t_efontKR_12_i,
    efontKR_14 = lgfx_font_id_t_efontKR_14,
    efontKR_14_b = lgfx_font_id_t_efontKR_14_b,
    efontKR_14_bi = lgfx_font_id_t_efontKR_14_bi,
    efontKR_14_i = lgfx_font_id_t_efontKR_14_i,
    efontKR_16 = lgfx_font_id_t_efontKR_16,
    efontKR_16_b = lgfx_font_id_t_efontKR_16_b,
    efontKR_16_bi = lgfx_font_id_t_efontKR_16_bi,
    efontKR_16_i = lgfx_font_id_t_efontKR_16_i,
    efontKR_24 = lgfx_font_id_t_efontKR_24,
    efontKR_24_b = lgfx_font_id_t_efontKR_24_b,
    efontKR_24_bi = lgfx_font_id_t_efontKR_24_bi,
    efontKR_24_i = lgfx_font_id_t_efontKR_24_i,
    efontTW_10 = lgfx_font_id_t_efontTW_10,
    efontTW_10_b = lgfx_font_id_t_efontTW_10_b,
    efontTW_10_bi = lgfx_font_id_t_efontTW_10_bi,
    efontTW_10_i = lgfx_font_id_t_efontTW_10_i,
    efontTW_12 = lgfx_font_id_t_efontTW_12,
    efontTW_12_b = lgfx_font_id_t_efontTW_12_b,
    efontTW_12_bi = lgfx_font_id_t_efontTW_12_bi,
    efontTW_12_i = lgfx_font_id_t_efontTW_12_i,
    efontTW_14 = lgfx_font_id_t_efontTW_14,
    efontTW_14_b = lgfx_font_id_t_efontTW_14_b,
    efontTW_14_bi = lgfx_font_id_t_efontTW_14_bi,
    efontTW_14_i = lgfx_font_id_t_efontTW_14_i,
    efontTW_16 = lgfx_font_id_t_efontTW_16,
    efontTW_16_b = lgfx_font_id_t_efontTW_16_b,
    efontTW_16_bi = lgfx_font_id_t_efontTW_16_bi,
    efontTW_16_i = lgfx_font_id_t_efontTW_16_i,
    efontTW_24 = lgfx_font_id_t_efontTW_24,
    efontTW_24_b = lgfx_font_id_t_efontTW_24_b,
    efontTW_24_bi = lgfx_font_id_t_efontTW_24_bi,
    efontTW_24_i = lgfx_font_id_t_efontTW_24_i,
}

#include "lgfx_c.h"

#define LGFX_USE_V1
#define LGFX_AUTODETECT
#include <LovyanGFX.hpp>
#include <stdint.h>

static LGFX gfx;
using namespace lgfx::v1;

lgfx_target_t lgfx_c_setup(void) 
{
    gfx.init();
    gfx.setEpdMode(epd_mode_t::epd_quality);
    return reinterpret_cast<lgfx_target_t>(static_cast<LovyanGFX*>(&gfx));
}

int32_t lgfx_c_width(lgfx_target_t target) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    return gfx->width();
}
int32_t lgfx_c_height(lgfx_target_t target) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    return gfx->height();
}

void lgfx_c_start_write(lgfx_target_t target) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->startWrite();
}
void lgfx_c_end_write(lgfx_target_t target) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->endWrite();
}

void lgfx_c_clear_rgb332(lgfx_target_t target, uint8_t color) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->clear(color);
}
void lgfx_c_clear_rgb888(lgfx_target_t target, uint32_t color) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->clear(color);
}

void lgfx_c_fill_rect_rgb332(lgfx_target_t target, int32_t left, int32_t top, int32_t width, int32_t height, uint8_t color) { 
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->fillRect(left, top, width, height, rgb332_t(color));
}
void lgfx_c_fill_rect_rgb888(lgfx_target_t target, int32_t left, int32_t top, int32_t width, int32_t height, uint32_t color) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->fillRect(left, top, width, height, rgb888_t(color));
}

void lgfx_c_draw_line_rgb332(lgfx_target_t target, int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint8_t color){
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->drawLine(x0, y0, x1, y1, color);
}
void lgfx_c_draw_line_rgb888(lgfx_target_t target, int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint32_t color){
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->drawLine(x0, y0, x1, y1, color);
}

void lgfx_c_push_image_grayscale(lgfx_target_t target, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->pushGrayscaleImage(x, y, w, h, data, color_depth_t::grayscale_8bit, TFT_WHITE, TFT_BLACK);
}
void lgfx_c_push_image_rgb332(lgfx_target_t target, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->pushImage(x, y, w, h, reinterpret_cast<const rgb332_t*>(data));
}
void lgfx_c_push_image_rgb888(lgfx_target_t target, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->pushImage(x, y, w, h, reinterpret_cast<const rgb888_t*>(data));
}

bool lgfx_c_draw_png(lgfx_target_t target, const uint8_t *data, uint32_t len, int32_t x, int32_t y, int32_t maxWidth, int32_t maxHeight, int32_t offX, int32_t offY, float scale_x, float scale_y, ::textdatum_t datum) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    return gfx->drawPng(data, len, x, y, maxWidth, maxHeight, offX, offY, scale_x, scale_y, static_cast<datum_t>(datum));
}

lgfx_target_t lgfx_c_create_sprite(lgfx_target_t target, int32_t w, int32_t h) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    auto sprite = new LGFX_Sprite(gfx);
    if( sprite == nullptr ) return nullptr;
    if( sprite->createSprite(w, h) == nullptr ) {
        delete sprite;
        return nullptr;
    }
    return reinterpret_cast<lgfx_target_t>(static_cast<LovyanGFX*>(sprite));
}
lgfx_target_t lgfx_c_create_sprite_static(lgfx_target_t target, int32_t w, int32_t h, void* buffer, uint8_t bpp) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    auto sprite = new LGFX_Sprite(gfx);
    if( sprite == nullptr ) return nullptr;
    sprite->setBuffer(buffer, w, h, bpp);
    return reinterpret_cast<lgfx_target_t>(static_cast<LovyanGFX*>(sprite));
}
void lgfx_c_push_sprite(lgfx_target_t target, int32_t x, int32_t y) {
    auto sprite = static_cast<LGFX_Sprite*>(reinterpret_cast<LovyanGFX*>(target));
    sprite->pushSprite(x, y);
}
void lgfx_c_delete_sprite(lgfx_target_t target) {
    if( target != nullptr ) {
        auto sprite = static_cast<LGFX_Sprite*>(reinterpret_cast<LovyanGFX*>(target));
        delete sprite;
    }
}

size_t lgfx_c_write(lgfx_target_t target, const uint8_t* buffer, size_t length) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    return gfx->write(buffer, length);
}
void lgfx_c_set_cursor(lgfx_target_t target, int32_t x, int32_t y) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->setCursor(x, y);
}
void lgfx_c_set_text_size(lgfx_target_t target, float sx, float sy) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->setTextSize(sx, sy);
}
size_t lgfx_c_draw_char_rgb332(lgfx_target_t target, int32_t x, int32_t y, uint16_t unicode, uint8_t color, uint8_t bg, float size_x, float size_y) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    return gfx->drawChar(x, y, unicode, color, bg, size_x, size_y);
}
size_t lgfx_c_draw_char_rgb888(lgfx_target_t target, int32_t x, int32_t y, uint16_t unicode, uint32_t color, uint32_t bg, float size_x, float size_y) {
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    return gfx->drawChar(x, y, unicode, color, bg, size_x, size_y);
}

static const lgfx::IFont* lgfx_c_get_font_inner(lgfx_font_id_t id) {
    switch(id) {
        case lgfx_font_id_t::Font0: return &lgfx::fonts::Font0;
        case lgfx_font_id_t::Font2: return &lgfx::fonts:: Font2;
        case lgfx_font_id_t::Font4: return &lgfx::fonts:: Font4;
        case lgfx_font_id_t::Font6: return &lgfx::fonts:: Font6;
        case lgfx_font_id_t::Font7: return &lgfx::fonts:: Font7;
        case lgfx_font_id_t::Font8: return &lgfx::fonts:: Font8;
        // case lgfx_font_id_t::Font8x8C64: return &lgfx::fonts::Font8x8C64;
        // case lgfx_font_id_t::AsciiFont8x16: return &lgfx::fonts::AsciiFont8x16;
        // case lgfx_font_id_t::AsciiFont24x48: return &lgfx::fonts::AsciiFont24x48;
        // case lgfx_font_id_t::TomThumb: return &lgfx::fonts::TomThumb                 ;
        // case lgfx_font_id_t::FreeMono9pt7b: return &lgfx::fonts::FreeMono9pt7b            ;
        // case lgfx_font_id_t::FreeMono12pt7b: return &lgfx::fonts::FreeMono12pt7b           ;
        // case lgfx_font_id_t::FreeMono18pt7b: return &lgfx::fonts::FreeMono18pt7b           ;
        // case lgfx_font_id_t::FreeMono24pt7b: return &lgfx::fonts::FreeMono24pt7b           ;
        // case lgfx_font_id_t::FreeMonoBold9pt7b: return &lgfx::fonts::FreeMonoBold9pt7b        ;
        // case lgfx_font_id_t::FreeMonoBold12pt7b: return &lgfx::fonts::FreeMonoBold12pt7b       ;
        // case lgfx_font_id_t::FreeMonoBold18pt7b: return &lgfx::fonts::FreeMonoBold18pt7b       ;
        // case lgfx_font_id_t::FreeMonoBold24pt7b: return &lgfx::fonts::FreeMonoBold24pt7b       ;
        // case lgfx_font_id_t::FreeMonoOblique9pt7b: return &lgfx::fonts::FreeMonoOblique9pt7b     ;
        // case lgfx_font_id_t::FreeMonoOblique12pt7b: return &lgfx::fonts::FreeMonoOblique12pt7b    ;
        // case lgfx_font_id_t::FreeMonoOblique18pt7b: return &lgfx::fonts::FreeMonoOblique18pt7b    ;
        // case lgfx_font_id_t::FreeMonoOblique24pt7b: return &lgfx::fonts::FreeMonoOblique24pt7b    ;
        // case lgfx_font_id_t::FreeMonoBoldOblique9pt7b: return &lgfx::fonts::FreeMonoBoldOblique9pt7b ;
        // case lgfx_font_id_t::FreeMonoBoldOblique12pt7b: return &lgfx::fonts::FreeMonoBoldOblique12pt7b;
        // case lgfx_font_id_t::FreeMonoBoldOblique18pt7b: return &lgfx::fonts::FreeMonoBoldOblique18pt7b;
        // case lgfx_font_id_t::FreeMonoBoldOblique24pt7b: return &lgfx::fonts::FreeMonoBoldOblique24pt7b;
        // case lgfx_font_id_t::FreeSans9pt7b: return &lgfx::fonts::FreeSans9pt7b            ;
        // case lgfx_font_id_t::FreeSans12pt7b: return &lgfx::fonts::FreeSans12pt7b           ;
        // case lgfx_font_id_t::FreeSans18pt7b: return &lgfx::fonts::FreeSans18pt7b           ;
        // case lgfx_font_id_t::FreeSans24pt7b: return &lgfx::fonts::FreeSans24pt7b           ;
        // case lgfx_font_id_t::FreeSansBold9pt7b: return &lgfx::fonts::FreeSansBold9pt7b        ;
        // case lgfx_font_id_t::FreeSansBold12pt7b: return &lgfx::fonts::FreeSansBold12pt7b       ;
        // case lgfx_font_id_t::FreeSansBold18pt7b: return &lgfx::fonts::FreeSansBold18pt7b       ;
        // case lgfx_font_id_t::FreeSansBold24pt7b: return &lgfx::fonts::FreeSansBold24pt7b       ;
        // case lgfx_font_id_t::FreeSansOblique9pt7b: return &lgfx::fonts::FreeSansOblique9pt7b     ;
        // case lgfx_font_id_t::FreeSansOblique12pt7b: return &lgfx::fonts::FreeSansOblique12pt7b    ;
        // case lgfx_font_id_t::FreeSansOblique18pt7b: return &lgfx::fonts::FreeSansOblique18pt7b    ;
        // case lgfx_font_id_t::FreeSansOblique24pt7b: return &lgfx::fonts::FreeSansOblique24pt7b    ;
        // case lgfx_font_id_t::FreeSansBoldOblique9pt7b: return &lgfx::fonts::FreeSansBoldOblique9pt7b ;
        // case lgfx_font_id_t::FreeSansBoldOblique12pt7b: return &lgfx::fonts::FreeSansBoldOblique12pt7b;
        // case lgfx_font_id_t::FreeSansBoldOblique18pt7b: return &lgfx::fonts::FreeSansBoldOblique18pt7b;
        // case lgfx_font_id_t::FreeSansBoldOblique24pt7b: return &lgfx::fonts::FreeSansBoldOblique24pt7b;
        // case lgfx_font_id_t::FreeSerif9pt7b: return &lgfx::fonts::FreeSerif9pt7b           ;
        // case lgfx_font_id_t::FreeSerif12pt7b: return &lgfx::fonts::FreeSerif12pt7b          ;
        // case lgfx_font_id_t::FreeSerif18pt7b: return &lgfx::fonts::FreeSerif18pt7b          ;
        // case lgfx_font_id_t::FreeSerif24pt7b: return &lgfx::fonts::FreeSerif24pt7b          ;
        // case lgfx_font_id_t::FreeSerifItalic9pt7b: return &lgfx::fonts::FreeSerifItalic9pt7b     ;
        // case lgfx_font_id_t::FreeSerifItalic12pt7b: return &lgfx::fonts::FreeSerifItalic12pt7b    ;
        // case lgfx_font_id_t::FreeSerifItalic18pt7b: return &lgfx::fonts::FreeSerifItalic18pt7b    ;
        // case lgfx_font_id_t::FreeSerifItalic24pt7b: return &lgfx::fonts::FreeSerifItalic24pt7b    ;
        // case lgfx_font_id_t::FreeSerifBold9pt7b: return &lgfx::fonts::FreeSerifBold9pt7b       ;
        // case lgfx_font_id_t::FreeSerifBold12pt7b: return &lgfx::fonts::FreeSerifBold12pt7b      ;
        // case lgfx_font_id_t::FreeSerifBold18pt7b: return &lgfx::fonts::FreeSerifBold18pt7b      ;
        // case lgfx_font_id_t::FreeSerifBold24pt7b: return &lgfx::fonts::FreeSerifBold24pt7b      ;
        // case lgfx_font_id_t::FreeSerifBoldItalic9pt7b: return &lgfx::fonts::FreeSerifBoldItalic9pt7b ;
        // case lgfx_font_id_t::FreeSerifBoldItalic12pt7b: return &lgfx::fonts::FreeSerifBoldItalic12pt7b;
        // case lgfx_font_id_t::FreeSerifBoldItalic18pt7b: return &lgfx::fonts::FreeSerifBoldItalic18pt7b;
        // case lgfx_font_id_t::FreeSerifBoldItalic24pt7b: return &lgfx::fonts::FreeSerifBoldItalic24pt7b;
        // case lgfx_font_id_t::Orbitron_Light_24: return &lgfx::fonts::Orbitron_Light_24;
        // case lgfx_font_id_t::Orbitron_Light_32: return &lgfx::fonts::Orbitron_Light_32;
        // case lgfx_font_id_t::Roboto_Thin_24: return &lgfx::fonts::Roboto_Thin_24   ;
        // case lgfx_font_id_t::Satisfy_24: return &lgfx::fonts::Satisfy_24       ;
        // case lgfx_font_id_t::Yellowtail_32: return &lgfx::fonts::Yellowtail_32    ;
        // case lgfx_font_id_t::DejaVu9: return &lgfx::fonts::DejaVu9 ;
        // case lgfx_font_id_t::DejaVu12: return &lgfx::fonts::DejaVu12;
        // case lgfx_font_id_t::DejaVu18: return &lgfx::fonts::DejaVu18;
        // case lgfx_font_id_t::DejaVu24: return &lgfx::fonts::DejaVu24;
        // case lgfx_font_id_t::DejaVu40: return &lgfx::fonts::DejaVu40;
        // case lgfx_font_id_t::DejaVu56: return &lgfx::fonts::DejaVu56;
        // case lgfx_font_id_t::DejaVu72: return &lgfx::fonts::DejaVu72;
        // case lgfx_font_id_t::lgfxJapanMincho_8  : return &lgfx::fonts::lgfxJapanMincho_8;
        // case lgfx_font_id_t::lgfxJapanMincho_12 : return &lgfx::fonts::lgfxJapanMincho_12;
        // case lgfx_font_id_t::lgfxJapanMincho_16 : return &lgfx::fonts::lgfxJapanMincho_16;
        // case lgfx_font_id_t::lgfxJapanMincho_20 : return &lgfx::fonts::lgfxJapanMincho_20;
        // case lgfx_font_id_t::lgfxJapanMincho_24 : return &lgfx::fonts::lgfxJapanMincho_24;
        // case lgfx_font_id_t::lgfxJapanMincho_28 : return &lgfx::fonts::lgfxJapanMincho_28;
        // case lgfx_font_id_t::lgfxJapanMincho_32 : return &lgfx::fonts::lgfxJapanMincho_32;
        // case lgfx_font_id_t::lgfxJapanMincho_36 : return &lgfx::fonts::lgfxJapanMincho_36;
        // case lgfx_font_id_t::lgfxJapanMincho_40 : return &lgfx::fonts::lgfxJapanMincho_40;
        // case lgfx_font_id_t::lgfxJapanMinchoP_8 : return &lgfx::fonts::lgfxJapanMinchoP_8;
        // case lgfx_font_id_t::lgfxJapanMinchoP_12: return &lgfx::fonts::lgfxJapanMinchoP_12;
        // case lgfx_font_id_t::lgfxJapanMinchoP_16: return &lgfx::fonts::lgfxJapanMinchoP_16;
        // case lgfx_font_id_t::lgfxJapanMinchoP_20: return &lgfx::fonts::lgfxJapanMinchoP_20;
        // case lgfx_font_id_t::lgfxJapanMinchoP_24: return &lgfx::fonts::lgfxJapanMinchoP_24;
        // case lgfx_font_id_t::lgfxJapanMinchoP_28: return &lgfx::fonts::lgfxJapanMinchoP_28;
        // case lgfx_font_id_t::lgfxJapanMinchoP_32: return &lgfx::fonts::lgfxJapanMinchoP_32;
        // case lgfx_font_id_t::lgfxJapanMinchoP_36: return &lgfx::fonts::lgfxJapanMinchoP_36;
        // case lgfx_font_id_t::lgfxJapanMinchoP_40: return &lgfx::fonts::lgfxJapanMinchoP_40;
        // case lgfx_font_id_t::lgfxJapanGothic_8  : return &lgfx::fonts::lgfxJapanGothic_8;
        // case lgfx_font_id_t::lgfxJapanGothic_12 : return &lgfx::fonts::lgfxJapanGothic_12;
        // case lgfx_font_id_t::lgfxJapanGothic_16 : return &lgfx::fonts::lgfxJapanGothic_16;
        // case lgfx_font_id_t::lgfxJapanGothic_20 : return &lgfx::fonts::lgfxJapanGothic_20;
        // case lgfx_font_id_t::lgfxJapanGothic_24 : return &lgfx::fonts::lgfxJapanGothic_24;
        // case lgfx_font_id_t::lgfxJapanGothic_28 : return &lgfx::fonts::lgfxJapanGothic_28;
        // case lgfx_font_id_t::lgfxJapanGothic_32 : return &lgfx::fonts::lgfxJapanGothic_32;
        // case lgfx_font_id_t::lgfxJapanGothic_36 : return &lgfx::fonts::lgfxJapanGothic_36;
        // case lgfx_font_id_t::lgfxJapanGothic_40 : return &lgfx::fonts::lgfxJapanGothic_40;
        // case lgfx_font_id_t::lgfxJapanGothicP_8 : return &lgfx::fonts::lgfxJapanGothicP_8;
        // case lgfx_font_id_t::lgfxJapanGothicP_12: return &lgfx::fonts::lgfxJapanGothicP_12;
        // case lgfx_font_id_t::lgfxJapanGothicP_16: return &lgfx::fonts::lgfxJapanGothicP_16;
        // case lgfx_font_id_t::lgfxJapanGothicP_20: return &lgfx::fonts::lgfxJapanGothicP_20;
        // case lgfx_font_id_t::lgfxJapanGothicP_24: return &lgfx::fonts::lgfxJapanGothicP_24;
        // case lgfx_font_id_t::lgfxJapanGothicP_28: return &lgfx::fonts::lgfxJapanGothicP_28;
        // case lgfx_font_id_t::lgfxJapanGothicP_32: return &lgfx::fonts::lgfxJapanGothicP_32;
        // case lgfx_font_id_t::lgfxJapanGothicP_36: return &lgfx::fonts::lgfxJapanGothicP_36;
        // case lgfx_font_id_t::lgfxJapanGothicP_40: return &lgfx::fonts::lgfxJapanGothicP_40;
        // case lgfx_font_id_t::efontCN_10   : return &lgfx::fonts::efontCN_10;
        // case lgfx_font_id_t::efontCN_10_b : return &lgfx::fonts::efontCN_10_b;
        // case lgfx_font_id_t::efontCN_10_bi: return &lgfx::fonts::efontCN_10_bi;
        // case lgfx_font_id_t::efontCN_10_i : return &lgfx::fonts::efontCN_10_i;
        // case lgfx_font_id_t::efontCN_12   : return &lgfx::fonts::efontCN_12;
        // case lgfx_font_id_t::efontCN_12_b : return &lgfx::fonts::efontCN_12_b;
        // case lgfx_font_id_t::efontCN_12_bi: return &lgfx::fonts::efontCN_12_bi;
        // case lgfx_font_id_t::efontCN_12_i : return &lgfx::fonts::efontCN_12_i;
        // case lgfx_font_id_t::efontCN_14   : return &lgfx::fonts::efontCN_14;
        // case lgfx_font_id_t::efontCN_14_b : return &lgfx::fonts::efontCN_14_b;
        // case lgfx_font_id_t::efontCN_14_bi: return &lgfx::fonts::efontCN_14_bi;
        // case lgfx_font_id_t::efontCN_14_i : return &lgfx::fonts::efontCN_14_i;
        // case lgfx_font_id_t::efontCN_16   : return &lgfx::fonts::efontCN_16;
        // case lgfx_font_id_t::efontCN_16_b : return &lgfx::fonts::efontCN_16_b;
        // case lgfx_font_id_t::efontCN_16_bi: return &lgfx::fonts::efontCN_16_bi;
        // case lgfx_font_id_t::efontCN_16_i : return &lgfx::fonts::efontCN_16_i;
        // case lgfx_font_id_t::efontCN_24   : return &lgfx::fonts::efontCN_24;
        // case lgfx_font_id_t::efontCN_24_b : return &lgfx::fonts::efontCN_24_b;
        // case lgfx_font_id_t::efontCN_24_bi: return &lgfx::fonts::efontCN_24_bi;
        // case lgfx_font_id_t::efontCN_24_i : return &lgfx::fonts::efontCN_24_i;
        // case lgfx_font_id_t::efontJA_10   : return &lgfx::fonts::efontJA_10;
        // case lgfx_font_id_t::efontJA_10_b : return &lgfx::fonts::efontJA_10_b;
        // case lgfx_font_id_t::efontJA_10_bi: return &lgfx::fonts::efontJA_10_bi;
        // case lgfx_font_id_t::efontJA_10_i : return &lgfx::fonts::efontJA_10_i;
        // case lgfx_font_id_t::efontJA_12   : return &lgfx::fonts::efontJA_12;
        // case lgfx_font_id_t::efontJA_12_b : return &lgfx::fonts::efontJA_12_b;
        // case lgfx_font_id_t::efontJA_12_bi: return &lgfx::fonts::efontJA_12_bi;
        // case lgfx_font_id_t::efontJA_12_i : return &lgfx::fonts::efontJA_12_i;
        // case lgfx_font_id_t::efontJA_14   : return &lgfx::fonts::efontJA_14;
        // case lgfx_font_id_t::efontJA_14_b : return &lgfx::fonts::efontJA_14_b;
        // case lgfx_font_id_t::efontJA_14_bi: return &lgfx::fonts::efontJA_14_bi;
        // case lgfx_font_id_t::efontJA_14_i : return &lgfx::fonts::efontJA_14_i;
        // case lgfx_font_id_t::efontJA_16   : return &lgfx::fonts::efontJA_16;
        // case lgfx_font_id_t::efontJA_16_b : return &lgfx::fonts::efontJA_16_b;
        // case lgfx_font_id_t::efontJA_16_bi: return &lgfx::fonts::efontJA_16_bi;
        // case lgfx_font_id_t::efontJA_16_i : return &lgfx::fonts::efontJA_16_i;
        // case lgfx_font_id_t::efontJA_24   : return &lgfx::fonts::efontJA_24;
        // case lgfx_font_id_t::efontJA_24_b : return &lgfx::fonts::efontJA_24_b;
        // case lgfx_font_id_t::efontJA_24_bi: return &lgfx::fonts::efontJA_24_bi;
        // case lgfx_font_id_t::efontJA_24_i : return &lgfx::fonts::efontJA_24_i;
        // case lgfx_font_id_t::efontKR_10   : return &lgfx::fonts::efontKR_10;
        // case lgfx_font_id_t::efontKR_10_b : return &lgfx::fonts::efontKR_10_b;
        // case lgfx_font_id_t::efontKR_10_bi: return &lgfx::fonts::efontKR_10_bi;
        // case lgfx_font_id_t::efontKR_10_i : return &lgfx::fonts::efontKR_10_i;
        // case lgfx_font_id_t::efontKR_12   : return &lgfx::fonts::efontKR_12;
        // case lgfx_font_id_t::efontKR_12_b : return &lgfx::fonts::efontKR_12_b;
        // case lgfx_font_id_t::efontKR_12_bi: return &lgfx::fonts::efontKR_12_bi;
        // case lgfx_font_id_t::efontKR_12_i : return &lgfx::fonts::efontKR_12_i;
        // case lgfx_font_id_t::efontKR_14   : return &lgfx::fonts::efontKR_14;
        // case lgfx_font_id_t::efontKR_14_b : return &lgfx::fonts::efontKR_14_b;
        // case lgfx_font_id_t::efontKR_14_bi: return &lgfx::fonts::efontKR_14_bi;
        // case lgfx_font_id_t::efontKR_14_i : return &lgfx::fonts::efontKR_14_i;
        // case lgfx_font_id_t::efontKR_16   : return &lgfx::fonts::efontKR_16;
        // case lgfx_font_id_t::efontKR_16_b : return &lgfx::fonts::efontKR_16_b;
        // case lgfx_font_id_t::efontKR_16_bi: return &lgfx::fonts::efontKR_16_bi;
        // case lgfx_font_id_t::efontKR_16_i : return &lgfx::fonts::efontKR_16_i;
        // case lgfx_font_id_t::efontKR_24   : return &lgfx::fonts::efontKR_24;
        // case lgfx_font_id_t::efontKR_24_b : return &lgfx::fonts::efontKR_24_b;
        // case lgfx_font_id_t::efontKR_24_bi: return &lgfx::fonts::efontKR_24_bi;
        // case lgfx_font_id_t::efontKR_24_i : return &lgfx::fonts::efontKR_24_i;
        // case lgfx_font_id_t::efontTW_10   : return &lgfx::fonts::efontTW_10;
        // case lgfx_font_id_t::efontTW_10_b : return &lgfx::fonts::efontTW_10_b;
        // case lgfx_font_id_t::efontTW_10_bi: return &lgfx::fonts::efontTW_10_bi;
        // case lgfx_font_id_t::efontTW_10_i : return &lgfx::fonts::efontTW_10_i;
        // case lgfx_font_id_t::efontTW_12   : return &lgfx::fonts::efontTW_12;
        // case lgfx_font_id_t::efontTW_12_b : return &lgfx::fonts::efontTW_12_b;
        // case lgfx_font_id_t::efontTW_12_bi: return &lgfx::fonts::efontTW_12_bi;
        // case lgfx_font_id_t::efontTW_12_i : return &lgfx::fonts::efontTW_12_i;
        // case lgfx_font_id_t::efontTW_14   : return &lgfx::fonts::efontTW_14;
        // case lgfx_font_id_t::efontTW_14_b : return &lgfx::fonts::efontTW_14_b;
        // case lgfx_font_id_t::efontTW_14_bi: return &lgfx::fonts::efontTW_14_bi;
        // case lgfx_font_id_t::efontTW_14_i : return &lgfx::fonts::efontTW_14_i;
        // case lgfx_font_id_t::efontTW_16   : return &lgfx::fonts::efontTW_16;
        // case lgfx_font_id_t::efontTW_16_b : return &lgfx::fonts::efontTW_16_b;
        // case lgfx_font_id_t::efontTW_16_bi: return &lgfx::fonts::efontTW_16_bi;
        // case lgfx_font_id_t::efontTW_16_i : return &lgfx::fonts::efontTW_16_i;
        // case lgfx_font_id_t::efontTW_24   : return &lgfx::fonts::efontTW_24;
        // case lgfx_font_id_t::efontTW_24_b : return &lgfx::fonts::efontTW_24_b;
        // case lgfx_font_id_t::efontTW_24_bi: return &lgfx::fonts::efontTW_24_bi;
        // case lgfx_font_id_t::efontTW_24_i : return &lgfx::fonts::efontTW_24_i;
        default: return nullptr;
    }
}

bool lgfx_c_set_font(lgfx_target_t target, lgfx_font_id_t id) {
    auto font = lgfx_c_get_font_inner(id);
    if( font == nullptr ) return false;
    auto gfx = reinterpret_cast<LovyanGFX*>(target);
    gfx->setFont(font);
    return true;
}

// lgfx_font_t lgfx_c_get_font(lgfx_font_id_t id) {
//     return reinterpret_cast<lgfx_font_t>(const_cast<lgfx::IFont*>(lgfx_c_get_font_inner(id)));
// }
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

enum textdatum_t
//  0:left   1:centre   2:right
//  0:top    4:middle   8:bottom   16:baseline
{ top_left        =  0  // Top left (default)
, top_center      =  1  // Top center
, top_centre      =  1  // Top center
, top_right       =  2  // Top right
, middle_left     =  4  // Middle left
, middle_center   =  5  // Middle center
, middle_centre   =  5  // Middle center
, middle_right    =  6  // Middle right
, bottom_left     =  8  // Bottom left
, bottom_center   =  9  // Bottom center
, bottom_centre   =  9  // Bottom center
, bottom_right    = 10  // Bottom right
, baseline_left   = 16  // Baseline left (Line the 'A' character would sit on)
, baseline_center = 17  // Baseline center
, baseline_centre = 17  // Baseline center
, baseline_right  = 18  // Baseline right
};

typedef struct lgfx_target *lgfx_target_t;

lgfx_target_t lgfx_c_setup(void);

int32_t lgfx_c_width(lgfx_target_t target);
int32_t lgfx_c_height(lgfx_target_t target);

void lgfx_c_clear_rgb332(lgfx_target_t target, uint8_t color);
void lgfx_c_clear_rgb888(lgfx_target_t target, uint32_t color);
void lgfx_c_fill_rect_rgb332(lgfx_target_t target, int32_t left, int32_t top, int32_t width, int32_t height, uint8_t color);
void lgfx_c_fill_rect_rgb888(lgfx_target_t target, int32_t left, int32_t top, int32_t width, int32_t height, uint32_t color);
void lgfx_c_draw_line_rgb332(lgfx_target_t target, int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint8_t color);
void lgfx_c_draw_line_rgb888(lgfx_target_t target, int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint32_t color);

void lgfx_c_push_image_grayscale(lgfx_target_t target, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data);
void lgfx_c_push_image_rgb332(lgfx_target_t target, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data);
void lgfx_c_push_image_rgb888(lgfx_target_t target, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data);

bool lgfx_c_draw_png(lgfx_target_t target, const uint8_t *data, uint32_t len, int32_t x, int32_t y, int32_t maxWidth, int32_t maxHeight, int32_t offX, int32_t offY, float scale_x, float scale_y, enum textdatum_t datum);

lgfx_target_t lgfx_c_create_sprite(lgfx_target_t target, int32_t w, int32_t h);
lgfx_target_t lgfx_c_create_sprite_static(lgfx_target_t target, int32_t w, int32_t h, void* buffer, uint8_t bpp);
void lgfx_c_push_sprite(lgfx_target_t target, int32_t x, int32_t y);
void lgfx_c_delete_sprite(lgfx_target_t target);

void lgfx_c_start_write(lgfx_target_t target);
void lgfx_c_end_write(lgfx_target_t target);

size_t lgfx_c_write(lgfx_target_t target, const uint8_t* buffer, size_t length);
void lgfx_c_set_cursor(lgfx_target_t target, int32_t x, int32_t y);
void lgfx_c_set_text_size(lgfx_target_t target, float sx, float sy);
size_t lgfx_c_draw_char_rgb332(lgfx_target_t target, int32_t x, int32_t y, uint16_t unicode, uint8_t color, uint8_t bg, float size_x, float size_y);
size_t lgfx_c_draw_char_rgb888(lgfx_target_t target, int32_t x, int32_t y, uint16_t unicode, uint32_t color, uint32_t bg, float size_x, float size_y);

bool lgfx_c_set_font(lgfx_target_t target, const void* font);

#ifdef __cplusplus
}
#endif
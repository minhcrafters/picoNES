use crate::{
    cart::Mirroring,
    ppu::framebuffer::Framebuffer,
    ppu::palette,
    ppu::{PPU, ScrollSegment},
};

struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect { x1, y1, x2, y2 }
    }
}

fn system_palette_color(ppu: &PPU, color_index: u8) -> (u8, u8, u8) {
    let mut idx = color_index & 0x3f;
    if ppu.mask.is_grayscale() {
        idx &= 0x30;
    }
    palette::SYSTEM_PALLETE[idx as usize]
}

fn nametable_slices<'a>(ppu: &'a PPU) -> [&'a [u8]; 4] {
    let (first, rest) = ppu.vram.split_at(0x400);
    let (second, _) = rest.split_at(0x400);
    match ppu.mapper.mirroring() {
        Mirroring::Vertical => [first, second, first, second],
        Mirroring::Horizontal => [first, first, second, second],
        Mirroring::SingleScreenLower => [first, first, first, first],
        Mirroring::SingleScreenUpper => [second, second, second, second],
        Mirroring::FourScreen => panic!("four screen mirroring is not supported"),
    }
}

fn bg_palette(ppu: &PPU, attribute_table: &[u8], tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_table_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = attribute_table[attr_table_idx];

    let pallet_idx = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("should not happen"),
    };

    let pallete_start: usize = 1 + (pallet_idx as usize) * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[pallete_start],
        ppu.palette_table[pallete_start + 1],
        ppu.palette_table[pallete_start + 2],
    ]
}

fn sprite_palette(ppu: &PPU, pallete_idx: u8) -> [u8; 4] {
    let start = 0x11 + (pallete_idx * 4) as usize;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

fn render_nametable(
    ppu: &PPU,
    frame: &mut Framebuffer,
    bg_priority: &mut [u8],
    nametable: &[u8],
    viewport: Rect,
    shift_x: isize,
    shift_y: isize,
    clip_y: (usize, usize),
) {
    if !ppu.mask.show_background() {
        return;
    }

    let attribute_table = &nametable[0x3c0..0x400];

    for i in 0..0x3c0 {
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile_idx = nametable[i] as u16;
        let mut tile = [0u8; 16];
        for i in 0..16 {
            tile[i] = ppu
                .mapper
                .read_chr(ppu.ctrl.bknd_pattern_addr() + tile_idx * 16 + i as u16);
        }
        let tile = &tile;
        let palette = bg_palette(ppu, attribute_table, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper >>= 1;
                lower >>= 1;
                let pixel_x = tile_column * 8 + x;
                let pixel_y = tile_row * 8 + y;

                if pixel_x >= viewport.x1
                    && pixel_x < viewport.x2
                    && pixel_y >= viewport.y1
                    && pixel_y < viewport.y2
                {
                    let target_x = shift_x + pixel_x as isize;
                    let target_y = shift_y + pixel_y as isize;

                    if target_x < 0
                        || target_x >= Framebuffer::WIDTH as isize
                        || target_y < 0
                        || target_y >= Framebuffer::HEIGHT as isize
                    {
                        continue;
                    }

                    if target_y < clip_y.0 as isize || target_y >= clip_y.1 as isize {
                        continue;
                    }

                    if !ppu.mask.leftmost_8pxl_background() && target_x < 8 {
                        continue;
                    }

                    let palette_index = match value {
                        0 => ppu.palette_table[0],
                        1 => palette[1],
                        2 => palette[2],
                        3 => palette[3],
                        _ => unreachable!(),
                    };

                    let rgb = system_palette_color(ppu, palette_index);

                    frame.set_pixel(target_x as usize, target_y as usize, rgb);
                    bg_priority[target_y as usize * Framebuffer::WIDTH + target_x as usize] = value;
                }
            }
        }
    }
}

fn render_sprites(ppu: &PPU, frame: &mut Framebuffer, bg_priority: &[u8]) {
    if !ppu.mask.show_sprites() {
        return;
    }

    let sprite_height = ppu.ctrl.sprite_size() as usize;

    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let sprite_y = (ppu.oam_data[i] as u16 + 1) as isize;
        if sprite_y >= Framebuffer::HEIGHT as isize + sprite_height as isize {
            continue;
        }

        let sprite_x = ppu.oam_data[i + 3] as isize;
        let tile_idx = ppu.oam_data[i + 1] as u16;
        let attributes = ppu.oam_data[i + 2];

        let priority_behind_bg = attributes & 0x20 != 0;
        let flip_horizontal = attributes & 0x40 != 0;
        let flip_vertical = attributes & 0x80 != 0;
        let pallette_idx = attributes & 0b11;
        let sprite_palette = sprite_palette(ppu, pallette_idx);

        let mut tile = [0u8; 32];
        if sprite_height == 16 {
            let base_tile = tile_idx & 0xFE;
            let bank = (tile_idx & 0x01) * 0x1000;
            for half in 0..2 {
                let addr = bank + (base_tile + half as u16) * 16;
                for byte in 0..16 {
                    tile[half * 16 + byte] = ppu.mapper.read_chr(addr + byte as u16);
                }
            }
        } else {
            let addr = ppu.ctrl.sprt_pattern_addr() + tile_idx * 16;
            for byte in 0..16 {
                tile[byte as usize] = ppu.mapper.read_chr(addr + byte as u16);
            }
        }

        for row in 0..sprite_height {
            let target_y = sprite_y + row as isize;
            if target_y < 0 || target_y >= Framebuffer::HEIGHT as isize {
                continue;
            }

            let source_row = if flip_vertical {
                sprite_height - 1 - row
            } else {
                row
            };

            let chunk = (source_row / 8) * 16;
            let plane0 = tile[chunk + (source_row % 8)];
            let plane1 = tile[chunk + (source_row % 8) + 8];

            for col in 0..8 {
                let bit = if flip_horizontal { col } else { 7 - col };
                let value = ((plane1 >> bit) & 1) << 1 | ((plane0 >> bit) & 1);
                if value == 0 {
                    continue;
                }

                let target_x = sprite_x + col as isize;

                if target_x < 0 || target_x >= Framebuffer::WIDTH as isize {
                    continue;
                }

                if !ppu.mask.leftmost_8pxl_sprite() && target_x < 8 {
                    continue;
                }

                let buffer_idx = target_y as usize * Framebuffer::WIDTH + target_x as usize;
                if priority_behind_bg && bg_priority[buffer_idx] != 0 {
                    continue;
                }

                let palette_index = sprite_palette[value as usize];
                let rgb = system_palette_color(ppu, palette_index);
                frame.set_pixel(target_x as usize, target_y as usize, rgb);
            }
        }
    }
}

pub fn render(ppu: &PPU, frame: &mut Framebuffer) {
    let universal_color = system_palette_color(ppu, ppu.palette_table[0]);
    for chunk in frame.data.chunks_mut(3) {
        chunk[0] = universal_color.0;
        chunk[1] = universal_color.1;
        chunk[2] = universal_color.2;
    }

    let mut bg_priority = vec![0u8; Framebuffer::WIDTH * Framebuffer::HEIGHT];

    let scroll_segments = ppu.scroll_segments();

    let fallback = if scroll_segments.is_empty() {
        let scroll_x = ppu.scroll.scroll_x();
        let scroll_y = ppu.scroll.scroll_y();
        let base_index = ppu.scroll.base_nametable();
        Some(ScrollSegment {
            start_scanline: 0,
            scroll_x,
            scroll_y,
            base_nametable: base_index,
            screen_origin: 0,
        })
    } else {
        None
    };

    let segments: &[ScrollSegment] = if let Some(fallback_segment) = fallback.as_ref() {
        std::slice::from_ref(fallback_segment)
    } else {
        scroll_segments
    };

    let name_tables = nametable_slices(ppu);

    if ppu.mask.show_background() {
        for (idx, segment) in segments.iter().enumerate() {
            let clip_start = segment.start_scanline.min(Framebuffer::HEIGHT);
            let clip_end = segments
                .get(idx + 1)
                .map(|next| next.start_scanline.min(Framebuffer::HEIGHT))
                .unwrap_or(Framebuffer::HEIGHT);

            if clip_start >= clip_end {
                continue;
            }

            let scroll_x = segment.scroll_x % Framebuffer::WIDTH;
            let scroll_y = segment.scroll_y % Framebuffer::HEIGHT;
            let base_index = segment.base_nametable & 0x03;

            let main_nametable = name_tables[base_index];
            let horizontal_nametable = name_tables[(base_index ^ 0x01) & 0x03];
            let vertical_nametable = name_tables[(base_index ^ 0x02) & 0x03];
            let diagonal_nametable = name_tables[(base_index ^ 0x03) & 0x03];

            let base_shift_x = -(scroll_x as isize);
            let base_shift_y = -(scroll_y as isize);
            let clip = (clip_start, clip_end);

            render_nametable(
                ppu,
                frame,
                &mut bg_priority,
                main_nametable,
                Rect::new(scroll_x, scroll_y, 256, 240),
                base_shift_x,
                base_shift_y,
                clip,
            );

            if scroll_x > 0 {
                render_nametable(
                    ppu,
                    frame,
                    &mut bg_priority,
                    horizontal_nametable,
                    Rect::new(0, scroll_y, scroll_x, 240),
                    base_shift_x + Framebuffer::WIDTH as isize,
                    base_shift_y,
                    clip,
                );
            }

            if scroll_y > 0 {
                render_nametable(
                    ppu,
                    frame,
                    &mut bg_priority,
                    vertical_nametable,
                    Rect::new(scroll_x, 0, 256, scroll_y),
                    base_shift_x,
                    base_shift_y + Framebuffer::HEIGHT as isize,
                    clip,
                );
            }

            if scroll_x > 0 && scroll_y > 0 {
                render_nametable(
                    ppu,
                    frame,
                    &mut bg_priority,
                    diagonal_nametable,
                    Rect::new(0, 0, scroll_x, scroll_y),
                    base_shift_x + Framebuffer::WIDTH as isize,
                    base_shift_y + Framebuffer::HEIGHT as isize,
                    clip,
                );
            }
        }
    }

    render_sprites(ppu, frame, &bg_priority);
}

use shipyard::{Get, UniqueView, View, World};

use crate::{components::CombatStats, map::Map, message::Messages, player::PlayerId};
use ruggle::{
    util::{Color, Position, Size},
    TileGrid, Tileset,
};

pub mod color {
    use super::Color;

    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
    pub const GRAY: Color = Color {
        r: 128,
        g: 128,
        b: 128,
    };
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
    pub const YELLOW: Color = Color {
        r: 255,
        g: 255,
        b: 0,
    };
    pub const MAGENTA: Color = Color {
        r: 255,
        g: 0,
        b: 255,
    };
    pub const CYAN: Color = Color {
        r: 0,
        g: 255,
        b: 255,
    };
    pub const ORANGE: Color = Color {
        r: 255,
        g: 166,
        b: 0,
    };
    pub const PURPLE: Color = Color {
        r: 128,
        g: 0,
        b: 128,
    };
    pub const PINK: Color = Color {
        r: 255,
        g: 191,
        b: 204,
    };

    pub const SELECTED_BG: Color = Color {
        r: 0,
        g: 128,
        b: 255,
    };
}

pub struct Options {
    pub map_zoom: u32,
    pub text_zoom: u32,
}

pub const MAP_GRID: usize = 0;
pub const UI_GRID: usize = 1;
pub const DEFAULT_MAP_TILESET: usize = 1;

fn draw_status_line(world: &World, grid: &mut TileGrid, y: i32) {
    let mut x = 2;

    let depth = format!(" Depth: {} ", world.borrow::<UniqueView<Map>>().depth);
    grid.set_draw_fg(color::YELLOW);
    grid.print_color((x, y), true, false, &depth);
    x += depth.len() as i32 + 1;

    let (hp, max_hp) = world.run(
        |player_id: UniqueView<PlayerId>, combat_stats: View<CombatStats>| {
            let player_stats = combat_stats.get(player_id.0);

            (player_stats.hp, player_stats.max_hp)
        },
    );
    let hp_string = format!(" HP: {} / {} ", hp, max_hp);
    grid.set_draw_fg(color::YELLOW);
    grid.print_color((x, y), true, false, &hp_string);
    x += hp_string.len() as i32 + 1;

    let hp_bar_length = grid.width() as i32 - x - 2;
    grid.set_draw_fg(color::RED);
    grid.draw_bar(false, (x, y), hp_bar_length, 0, hp, max_hp, true, false);
}

fn draw_messages(world: &World, grid: &mut TileGrid, active: bool, min_y: i32, max_y: i32) {
    world.run(|messages: UniqueView<Messages>| {
        grid.set_draw_fg(if active { color::WHITE } else { color::GRAY });
        for (y, message) in (min_y..=max_y).zip(messages.rev_iter()) {
            grid.put_color((0, y), true, false, '>');
            grid.print_color((2, y), true, false, message);
        }
    });
}

pub fn draw_ui(world: &World, grid: &mut TileGrid, prompt: Option<&str>) {
    let w = grid.width() as i32;
    let h = grid.height() as i32;

    grid.set_draw_fg(color::WHITE);
    for x in 0..w {
        grid.put_color((x, 0), true, false, '─');
    }

    draw_status_line(world, grid, 0);

    if let Some(prompt) = prompt {
        grid.set_draw_fg(color::WHITE);
        grid.print_color((2, 1), true, false, prompt);
        draw_messages(world, grid, false, 2, h - 1);
    } else {
        draw_messages(world, grid, true, 1, h);
    }
}

/// Prepares grid 0 and grid 1 to display the dungeon map and user interface respectively.
pub fn prepare_main_grids(
    world: &World,
    grids: &mut Vec<TileGrid>,
    tilesets: &[Tileset],
    window_size: Size,
) {
    let map_tileset = &tilesets[grids
        .get(MAP_GRID)
        .map_or(DEFAULT_MAP_TILESET, TileGrid::tileset)];
    let ui_tileset = &tilesets[grids.get(UI_GRID).map_or(0, TileGrid::tileset)];
    let Options {
        map_zoom,
        text_zoom,
    } = *world.borrow::<UniqueView<Options>>();

    let new_ui_size = Size {
        w: (window_size.w / (ui_tileset.tile_width() * text_zoom)).max(40),
        h: 5,
    };
    let new_ui_px_h = new_ui_size.h * ui_tileset.tile_height() * text_zoom;

    // 17 == standard field of view range * 2 + 1
    let mut new_map_w = (window_size.w / (map_tileset.tile_width() * map_zoom)).max(17);
    if window_size.w % (map_tileset.tile_width() * map_zoom) > 0 {
        // Fill to the edge of the screen.
        new_map_w += 1;
    }
    if new_map_w & 1 == 0 {
        // Ensure a single center tile exists using an odd number of tiles.
        new_map_w += 1;
    }

    // 17 == standard field of view range * 2 + 1
    let mut new_map_h = (window_size.h.saturating_sub(new_ui_px_h)
        / (map_tileset.tile_height() * map_zoom))
        .max(17);
    if window_size.h % (map_tileset.tile_height() * map_zoom) > 0 {
        // Fill to the edge of the screen.
        new_map_h += 1;
    }
    if new_map_h & 1 == 0 {
        // Ensure a single center tile exists using an odd number of tiles.
        new_map_h += 1;
    }

    let new_map_size = Size {
        w: new_map_w,
        h: new_map_h,
    };

    if !grids.is_empty() {
        grids[MAP_GRID].resize(new_map_size);
        grids[UI_GRID].resize(new_ui_size);
    } else {
        grids.push(TileGrid::new(new_map_size, tilesets, DEFAULT_MAP_TILESET));
        grids.push(TileGrid::new(new_ui_size, tilesets, 0));
        grids[MAP_GRID].view.clear_color = None;
        grids[UI_GRID].view.clear_color = Some(color::BLACK);
    }

    grids[MAP_GRID].view_centered(
        tilesets,
        map_zoom,
        Position { x: 0, y: 0 },
        Size {
            w: window_size.w,
            h: window_size.h.saturating_sub(new_ui_px_h).max(1),
        },
    );
    grids[MAP_GRID].view.zoom = map_zoom;

    grids[UI_GRID].view.pos = Position {
        x: 0,
        y: window_size.h.saturating_sub(new_ui_px_h) as i32,
    };
    grids[UI_GRID].view.size = Size {
        w: window_size.w,
        h: new_ui_px_h,
    };
    grids[UI_GRID].view.zoom = text_zoom;
}

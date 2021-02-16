use piston::input::{Button, Key};
use shipyard::{Get, UniqueView, View, World};
use std::collections::HashSet;

use crate::{
    components::{FieldOfView, Item, Monster, Name, Player, Position},
    map::Map,
    player::PlayerId,
    render, ui,
};
use ruggle::{CharGrid, InputBuffer, InputEvent};

use super::{
    yes_no_dialog::{YesNoDialogMode, YesNoDialogModeResult},
    ModeControl, ModeResult, ModeUpdate,
};

pub enum TargetModeResult {
    Cancelled,
    Target { x: i32, y: i32 },
}

pub struct TargetMode {
    for_what: String,
    center: (i32, i32), // x, y
    range: i32,
    radius: i32,
    valid: HashSet<(i32, i32)>,
    cursor: (i32, i32), // x, y
    warn_self: bool,
}

fn dist2((x1, y1): (i32, i32), (x2, y2): (i32, i32)) -> i32 {
    (x2 - x1).pow(2) + (y2 - y1).pow(2)
}

/// Pick a target position within a certain range of the player.
impl TargetMode {
    pub fn new(world: &World, for_what: String, range: i32, radius: i32, warn_self: bool) -> Self {
        assert!(range >= 0);
        assert!(radius >= 0);

        let player_pos: (i32, i32) = world.run(
            |player_id: UniqueView<PlayerId>, positions: View<Position>| {
                positions.get(player_id.0).into()
            },
        );

        let valid = world.run(|player_id: UniqueView<PlayerId>, fovs: View<FieldOfView>| {
            // Add 0.5 to the range to prevent 'bumps' at the edge of the range circle.
            let max_dist2 = range * (range + 1);
            fovs.get(player_id.0)
                .iter()
                .filter(|pos| dist2(*pos, player_pos) <= max_dist2)
                .collect::<HashSet<_>>()
        });

        // Default to the closest monster position, or the player if no monsters are present.
        let cursor = valid
            .iter()
            .filter(|(x, y)| {
                world
                    .borrow::<UniqueView<Map>>()
                    .iter_entities_at(*x, *y)
                    .any(|id| world.borrow::<View<Monster>>().contains(id))
            })
            .min_by_key(|pos| dist2(**pos, player_pos))
            .copied()
            .unwrap_or(player_pos);

        Self {
            for_what,
            center: player_pos,
            range,
            radius,
            valid,
            cursor,
            warn_self,
        }
    }

    pub fn update(
        &mut self,
        _world: &World,
        inputs: &mut InputBuffer,
        pop_result: &Option<ModeResult>,
    ) -> (ModeControl, ModeUpdate) {
        if let Some(result) = pop_result {
            return match result {
                ModeResult::YesNoDialogModeResult(result) => match result {
                    YesNoDialogModeResult::Yes => (
                        ModeControl::Pop(
                            TargetModeResult::Target {
                                x: self.cursor.0,
                                y: self.cursor.1,
                            }
                            .into(),
                        ),
                        ModeUpdate::Immediate,
                    ),
                    YesNoDialogModeResult::No => (ModeControl::Stay, ModeUpdate::WaitForEvent),
                },
                _ => (ModeControl::Stay, ModeUpdate::WaitForEvent),
            };
        }

        inputs.prepare_input();

        if let Some(InputEvent::Press(Button::Keyboard(key))) = inputs.get_input() {
            let min_x = self.center.0 - self.range;
            let max_x = self.center.0 + self.range;
            let min_y = self.center.1 - self.range;
            let max_y = self.center.1 + self.range;

            match key {
                Key::H | Key::NumPad4 | Key::Left => {
                    self.cursor.0 = std::cmp::max(min_x, self.cursor.0 - 1);
                }
                Key::J | Key::NumPad2 | Key::Down => {
                    self.cursor.1 = std::cmp::min(max_y, self.cursor.1 + 1);
                }
                Key::K | Key::NumPad8 | Key::Up => {
                    self.cursor.1 = std::cmp::max(min_y, self.cursor.1 - 1);
                }
                Key::L | Key::NumPad6 | Key::Right => {
                    self.cursor.0 = std::cmp::min(max_x, self.cursor.0 + 1);
                }
                Key::Y | Key::NumPad7 => {
                    if self.cursor.0 > min_x && self.cursor.1 > min_y {
                        self.cursor.0 -= 1;
                        self.cursor.1 -= 1;
                    }
                }
                Key::U | Key::NumPad9 => {
                    if self.cursor.0 < max_x && self.cursor.1 > min_y {
                        self.cursor.0 += 1;
                        self.cursor.1 -= 1;
                    }
                }
                Key::B | Key::NumPad1 => {
                    if self.cursor.0 > min_x && self.cursor.1 < max_y {
                        self.cursor.0 -= 1;
                        self.cursor.1 += 1;
                    }
                }
                Key::N | Key::NumPad3 => {
                    if self.cursor.0 < max_x && self.cursor.1 < max_y {
                        self.cursor.0 += 1;
                        self.cursor.1 += 1;
                    }
                }
                Key::Escape => {
                    return (
                        ModeControl::Pop(TargetModeResult::Cancelled.into()),
                        ModeUpdate::Immediate,
                    )
                }
                Key::Return => {
                    if self.valid.contains(&self.cursor) {
                        let result = if self.warn_self
                            && dist2(self.cursor, self.center) <= self.radius * (self.radius + 1)
                        {
                            inputs.clear_input();
                            ModeControl::Push(
                                YesNoDialogMode::new(
                                    format!(
                                        "Really {} yourself?",
                                        if self.cursor == self.center {
                                            "target"
                                        } else {
                                            "include"
                                        },
                                    ),
                                    false,
                                )
                                .into(),
                            )
                        } else {
                            ModeControl::Pop(
                                TargetModeResult::Target {
                                    x: self.cursor.0,
                                    y: self.cursor.1,
                                }
                                .into(),
                            )
                        };

                        return (result, ModeUpdate::Immediate);
                    }
                }
                _ => {}
            }
        }

        (ModeControl::Stay, ModeUpdate::WaitForEvent)
    }

    pub fn draw(&self, world: &World, grid: &mut CharGrid, active: bool) {
        render::draw_map(world, grid, active);
        render::draw_renderables(world, grid, active);

        let cx = grid.size_cells()[0] / 2;
        let cy = (grid.size_cells()[1] - ui::HUD_LINES) / 2;
        let (px, py) = world.run(
            |player_id: UniqueView<PlayerId>, positions: View<Position>| {
                positions.get(player_id.0).into()
            },
        );
        let target_bg = ui::recolor(ui::color::BLUE, active);
        let aoe_bg = ui::recolor(ui::color::PURPLE, active);
        let radius2 = self.radius * (self.radius + 1);

        // Highlight targetable spaces.
        for y in (self.center.1 - self.range)..=(self.center.1 + self.range) {
            for x in (self.center.0 - self.range)..=(self.center.0 + self.range) {
                if self.valid.contains(&(x, y)) {
                    grid.set_bg([x - px + cx, y - py + cy], target_bg);
                }
            }
        }

        // Highlight area of effect.
        for y in (self.cursor.1 - self.radius)..=(self.cursor.1 + self.radius) {
            for x in (self.cursor.0 - self.radius)..=(self.cursor.0 + self.radius) {
                if dist2((x, y), self.cursor) <= radius2 {
                    grid.set_bg([x - px + cx, y - py + cy], aoe_bg);
                }
            }
        }

        // Highlight cursor position.
        grid.set_bg(
            [self.cursor.0 - px + cx, self.cursor.1 - py + cy],
            ui::recolor(ui::color::MAGENTA, active),
        );

        // Describe the location that the cursor is positioned at.
        let cursor_desc = if self.valid.contains(&self.cursor) {
            let (map, items, monsters, names, players) = world.borrow::<(
                UniqueView<Map>,
                View<Item>,
                View<Monster>,
                View<Name>,
                View<Player>,
            )>();
            let entities_at = || map.iter_entities_at(self.cursor.0, self.cursor.1);
            let monster_id = entities_at().find(|id| monsters.contains(*id));

            if let Some(monster_id) = monster_id {
                names.get(monster_id).0.clone()
            } else {
                let player_id = entities_at().find(|id| players.contains(*id));

                if let Some(player_id) = player_id {
                    names.get(player_id).0.clone()
                } else {
                    let item_count = entities_at().filter(|id| items.contains(*id)).count();

                    #[allow(clippy::comparison_chain)]
                    if item_count == 1 {
                        let item_id = entities_at().find(|id| items.contains(*id));

                        names.get(item_id.unwrap()).0.clone()
                    } else if item_count > 1 {
                        format!("{} items", item_count)
                    } else {
                        map.get_tile(self.cursor.0, self.cursor.1).to_string()
                    }
                }
            }
        } else {
            "Out of range".to_string()
        };

        ui::draw_ui(
            world,
            grid,
            active,
            Some(&format!(
                "Pick target for {}: {}",
                self.for_what, cursor_desc
            )),
        );
    }
}

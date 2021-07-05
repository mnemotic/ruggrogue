use shipyard::{EntityId, Get, UniqueView, View, World};

use crate::{
    components::{Name, Renderable},
    gamekey::{self, GameKey},
    gamesym::GameSym,
    ui::{self, Options},
};
use ruggle::{
    util::{Color, Size},
    InputBuffer, InputEvent, KeyMods, TileGrid, Tileset,
};

use super::{ModeControl, ModeResult, ModeUpdate};

const CANCEL: &str = "[ Cancel ]";

pub enum EquipmentActionModeResult {
    AppQuit,
    Cancelled,
    RemoveEquipment(EntityId),
    DropEquipment(EntityId),
}

enum SubSection {
    Actions,
    Cancel,
}

#[allow(clippy::enum_variant_names)]
#[derive(Copy, Clone)]
enum EquipmentAction {
    RemoveEquipment,
    DropEquipment,
}

impl EquipmentAction {
    fn name(&self) -> &'static str {
        match self {
            EquipmentAction::RemoveEquipment => "[ Remove ]",
            EquipmentAction::DropEquipment => "[ Drop ]",
        }
    }
}

pub struct EquipmentActionMode {
    item_id: EntityId,
    inner_width: i32,
    actions: Vec<EquipmentAction>,
    subsection: SubSection,
    selection: i32,
}

/// Show a menu of actions for an item currently equipped by the player.
impl EquipmentActionMode {
    pub fn new(world: &World, item_id: EntityId) -> Self {
        let actions = [
            EquipmentAction::RemoveEquipment,
            EquipmentAction::DropEquipment,
        ]
        .iter()
        .copied()
        .collect::<Vec<_>>();
        let subsection = if actions.is_empty() {
            SubSection::Cancel
        } else {
            SubSection::Actions
        };
        let item_width = world.borrow::<View<Name>>().get(item_id).0.len();
        let inner_width = 2 + item_width
            .max(CANCEL.len())
            .max(actions.iter().map(|a| a.name().len()).max().unwrap_or(0));

        Self {
            item_id,
            inner_width: inner_width as i32,
            actions,
            subsection,
            selection: 0,
        }
    }

    pub fn prepare_grids(
        &self,
        world: &World,
        grids: &mut Vec<TileGrid<GameSym>>,
        tilesets: &[Tileset<GameSym>],
        window_size: Size,
    ) {
        let Options {
            font, text_zoom, ..
        } = *world.borrow::<UniqueView<Options>>();
        let new_grid_size = Size {
            w: 4 + self.inner_width as u32,
            h: 8 + self.actions.len() as u32,
        };

        if !grids.is_empty() {
            grids[0].resize(new_grid_size);
        } else {
            grids.push(TileGrid::new(new_grid_size, tilesets, font as usize));
            grids[0].view.clear_color = None;
        }

        grids[0].set_tileset(tilesets, font as usize);
        grids[0].view_centered(tilesets, text_zoom, (0, 0).into(), window_size);
        grids[0].view.zoom = text_zoom;
    }

    fn confirm_action(&self) -> (ModeControl, ModeUpdate) {
        let result = match self.subsection {
            SubSection::Actions => match self.actions[self.selection as usize] {
                EquipmentAction::RemoveEquipment => {
                    EquipmentActionModeResult::RemoveEquipment(self.item_id)
                }
                EquipmentAction::DropEquipment => {
                    EquipmentActionModeResult::DropEquipment(self.item_id)
                }
            },
            SubSection::Cancel => EquipmentActionModeResult::Cancelled,
        };

        (ModeControl::Pop(result.into()), ModeUpdate::Immediate)
    }

    pub fn update(
        &mut self,
        _world: &World,
        inputs: &mut InputBuffer,
        _grids: &[TileGrid<GameSym>],
        _pop_result: &Option<ModeResult>,
    ) -> (ModeControl, ModeUpdate) {
        inputs.prepare_input();

        if let Some(InputEvent::AppQuit) = inputs.get_input() {
            return (
                ModeControl::Pop(EquipmentActionModeResult::AppQuit.into()),
                ModeUpdate::Immediate,
            );
        } else if let Some(InputEvent::Press(keycode)) = inputs.get_input() {
            match gamekey::from_keycode(keycode, inputs.get_mods(KeyMods::SHIFT)) {
                GameKey::Up => match self.subsection {
                    SubSection::Actions => {
                        if self.selection > 0 {
                            self.selection -= 1;
                        } else {
                            self.subsection = SubSection::Cancel;
                        }
                    }
                    SubSection::Cancel => {
                        if !self.actions.is_empty() {
                            self.subsection = SubSection::Actions;
                            self.selection = self.actions.len() as i32 - 1;
                        }
                    }
                },
                GameKey::Down => match self.subsection {
                    SubSection::Actions => {
                        if self.selection < self.actions.len() as i32 - 1 {
                            self.selection += 1;
                        } else {
                            self.subsection = SubSection::Cancel;
                        }
                    }
                    SubSection::Cancel => {
                        if !self.actions.is_empty() {
                            self.subsection = SubSection::Actions;
                            self.selection = 0;
                        }
                    }
                },
                GameKey::Cancel => {
                    return (
                        ModeControl::Pop(EquipmentActionModeResult::Cancelled.into()),
                        ModeUpdate::Immediate,
                    )
                }
                GameKey::Confirm => return self.confirm_action(),
                _ => {}
            }
        }

        (ModeControl::Stay, ModeUpdate::WaitForEvent)
    }

    pub fn draw(&self, world: &World, grids: &mut [TileGrid<GameSym>], active: bool) {
        let grid = &mut grids[0];
        let fg = Color::WHITE;
        let bg = Color::BLACK;
        let selected_bg = ui::SELECTED_BG;

        grid.view.color_mod = if active { Color::WHITE } else { Color::GRAY };

        grid.draw_box((0, 0), (grid.width(), grid.height()), fg, bg);

        {
            let names = world.borrow::<View<Name>>();
            let renderables = world.borrow::<View<Renderable>>();
            let render = renderables.get(self.item_id);

            grid.put_sym_color((2, 2), render.sym, render.fg, render.bg);
            grid.print_color((4, 2), &names.get(self.item_id).0, true, fg, bg);
        }

        for (i, action) in self.actions.iter().enumerate() {
            grid.print_color(
                (4, 4 + i as i32),
                action.name(),
                true,
                fg,
                if matches!(self.subsection, SubSection::Actions) && i as i32 == self.selection {
                    selected_bg
                } else {
                    bg
                },
            );
        }

        grid.print_color(
            (4, grid.height() as i32 - 3),
            CANCEL,
            true,
            fg,
            if matches!(self.subsection, SubSection::Cancel) {
                selected_bg
            } else {
                bg
            },
        );
    }
}

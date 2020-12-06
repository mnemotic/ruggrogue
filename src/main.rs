mod components;
mod map;
mod player;
mod rect;
mod vision;

use rand::{thread_rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use shipyard::{EntitiesViewMut, EntityId, Get, IntoIter, UniqueView, View, ViewMut, World};
use std::path::PathBuf;

use crate::{
    components::{FieldOfView, Player, PlayerId, Position, Renderable},
    map::{draw_map, Map},
    player::player_input,
};
use ruggle::{CharGrid, RunSettings};

pub struct RuggleRng(Pcg64Mcg);

fn spawn_player(
    mut entities: EntitiesViewMut,
    mut players: ViewMut<Player>,
    mut positions: ViewMut<Position>,
    mut renderables: ViewMut<Renderable>,
    mut fovs: ViewMut<FieldOfView>,
) -> EntityId {
    entities.add_entity(
        (&mut players, &mut positions, &mut renderables, &mut fovs),
        (
            Player {},
            Position { x: 0, y: 0 },
            Renderable {
                ch: '@',
                fg: [1., 1., 0., 1.],
                bg: [0., 0., 0., 1.],
            },
            FieldOfView::new(8),
        ),
    )
}

fn draw_renderables(world: &World, grid: &mut CharGrid) {
    world.run(
        |player: UniqueView<PlayerId>, positions: View<Position>, renderables: View<Renderable>| {
            let (x, y) = positions.get(player.0).into();

            for (pos, render) in (&positions, &renderables).iter() {
                grid.put_color(
                    [pos.x - x + 40, pos.y - y + 18],
                    Some(render.fg),
                    Some(render.bg),
                    render.ch,
                );
            }
        },
    );
}

fn main() {
    let world = World::new();

    world.add_unique(RuggleRng(Pcg64Mcg::from_rng(thread_rng()).unwrap()));

    world.add_unique(Map::new(80, 50));
    world.run(map::generate_rooms_and_corridors);

    world.add_unique(PlayerId(world.run(spawn_player)));
    world.run(map::place_player_in_first_room);

    world.run(vision::recalculate_fields_of_view);

    let settings = RunSettings {
        title: "Ruggle".to_string(),
        grid_size: [80, 36],
        font_path: PathBuf::from("assets/gohufont-uni-14.ttf"),
        font_size: 14.0,
        min_fps: 30,
        max_fps: 60,
        start_inactive: true,
    };

    ruggle::run(settings, |mut inputs, mut grid| {
        if player_input(&world, &mut inputs) {
            world.run(vision::recalculate_fields_of_view);
        }

        grid.clear();
        draw_map(&world, &mut grid);
        draw_renderables(&world, &mut grid);

        false
    });
}

use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};
use shipyard::{EntitiesViewMut, EntityId, UniqueViewMut, ViewMut, World};

use crate::{
    components::{
        BlocksTile, CombatStats, FieldOfView, Monster, Name, Player, Position, Renderable,
    },
    map::Map,
    rect::Rect,
    RuggleRng,
};

pub fn spawn_player(
    mut map: UniqueViewMut<Map>,
    mut entities: EntitiesViewMut,
    mut combat_stats: ViewMut<CombatStats>,
    mut fovs: ViewMut<FieldOfView>,
    mut names: ViewMut<Name>,
    mut players: ViewMut<Player>,
    mut positions: ViewMut<Position>,
    mut renderables: ViewMut<Renderable>,
) -> EntityId {
    let player_id = entities.add_entity(
        (
            &mut players,
            &mut combat_stats,
            &mut fovs,
            &mut names,
            &mut positions,
            &mut renderables,
        ),
        (
            Player {},
            CombatStats {
                max_hp: 30,
                hp: 30,
                defense: 2,
                power: 5,
            },
            FieldOfView::new(8),
            Name("Player".into()),
            Position { x: 0, y: 0 },
            Renderable {
                ch: '@',
                fg: [1., 1., 0., 1.],
                bg: [0., 0., 0., 1.],
            },
        ),
    );

    map.place_entity(player_id, (0, 0), false);

    player_id
}

fn spawn_monster(world: &World, pos: (i32, i32), ch: char, name: String, fg: &[f32; 4]) {
    world.run(
        |mut map: UniqueViewMut<Map>,
         mut entities: EntitiesViewMut,
         mut blocks: ViewMut<BlocksTile>,
         mut combat_stats: ViewMut<CombatStats>,
         mut fovs: ViewMut<FieldOfView>,
         mut monsters: ViewMut<Monster>,
         mut names: ViewMut<Name>,
         mut positions: ViewMut<Position>,
         mut renderables: ViewMut<Renderable>| {
            let monster_id = entities.add_entity(
                (
                    &mut monsters,
                    &mut blocks,
                    &mut combat_stats,
                    &mut fovs,
                    &mut names,
                    &mut positions,
                    &mut renderables,
                ),
                (
                    Monster {},
                    BlocksTile {},
                    CombatStats {
                        max_hp: 16,
                        hp: 16,
                        defense: 1,
                        power: 4,
                    },
                    FieldOfView::new(8),
                    Name(name),
                    pos.into(),
                    Renderable {
                        ch,
                        fg: *fg,
                        bg: [0., 0., 0., 1.],
                    },
                ),
            );

            map.place_entity(monster_id, pos, true);
        },
    );
}

fn spawn_random_monster_at(world: &World, pos: (i32, i32)) {
    let choice = world.run(|mut rng: UniqueViewMut<RuggleRng>| {
        [
            ('g', "Goblin", [0.5, 0.9, 0.2, 1.]),
            ('o', "Orc", [0.9, 0.3, 0.2, 1.]),
        ]
        .choose(&mut rng.0)
    });

    if let Some((ch, name, fg)) = choice {
        spawn_monster(world, pos, *ch, name.to_string(), fg);
    }
}

fn fill_room_with_spawns(world: &World, room: &Rect) {
    if world.run(|mut rng: UniqueViewMut<RuggleRng>| rng.0.gen_ratio(1, 2)) {
        let positions = world.run(|mut rng: UniqueViewMut<RuggleRng>| {
            let num = rng.0.gen_range(1, 4);
            room.iter_xy().choose_multiple(&mut rng.0, num)
        });

        for pos in positions {
            spawn_random_monster_at(world, pos);
        }
    }
}

pub fn fill_rooms_with_spawns(world: &World) {
    let rooms =
        world.run(|map: UniqueViewMut<Map>| map.rooms.iter().skip(1).copied().collect::<Vec<_>>());

    for room in rooms {
        fill_room_with_spawns(world, &room);
    }
}

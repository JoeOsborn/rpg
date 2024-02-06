use assets_manager::{asset::Png, AssetCache};
use frenderer::{
    input::{Input, Key},
    sprites::{Camera2D, SheetRegion, Transform},
    wgpu, Renderer,
};
mod geom;
mod grid;
use geom::*;

#[derive(Debug, PartialEq, Eq)]
enum EntityType {
    Player,
    // which dialog to use
    Npc(usize),
    // which level, x in dest level, y in dest level
    Door(String, u16, u16),
}

#[derive(Clone, Copy, Debug)]
struct TileData {
    solid: bool,
    sheet_region: SheetRegion,
}

#[derive(Debug)]
struct Tileset {
    tiles: Vec<TileData>,
}
impl std::ops::Index<usize> for Tileset {
    type Output = TileData;
    fn index(&self, index: usize) -> &Self::Output {
        &self.tiles[index]
    }
}

mod level;
use level::Level;
struct Game {
    levels: Vec<Level>,
    mode: GameMode,
    dialogs: Vec<String>,
    active_dialog: Option<usize>,
    current_level: usize,
    npcs: Vec<(Vec2, usize)>,
    doors: Vec<(String, Vec2, Vec2)>,
    player: Vec2, // player, entities, other dynamic info here
    font: frenderer::bitfont::BitFont,
    window: frenderer::nineslice::NineSlice,
}

// Feel free to change this if you use a different tilesheet
const TILE_SZ: usize = 16;
const W: usize = 320;
const H: usize = 240;

const WIND_W: f32 = 288.0;
const WIND_H: f32 = 112.0;
const WIND_X: f32 = (W as f32 - WIND_W) / 2.0;
const WIND_Y: f32 = H as f32 - 16.0 - WIND_H;

const DLG_X: f32 = (W as f32 - WIND_W) / 2.0 + 16.0;
const DLG_Y: f32 = H as f32 - 16.0 - 16.0;

const DOOR: SheetRegion = SheetRegion::new(0, 561, 34, 15, TILE_SZ as i16, TILE_SZ as i16);
const NPC: SheetRegion = SheetRegion::new(0, 0, 714, 14, TILE_SZ as i16, TILE_SZ as i16);
const PLAYER: SheetRegion = SheetRegion::new(0, 0, 578, 14, TILE_SZ as i16, TILE_SZ as i16);

// TODO: point: (style) add two more rooms
// TODO: point: (style) transition animation between rooms
// TODO: point: (structure) NPC dialog contains yes/no or other choices (modify dialog.txt and use numbers to point to the "next" dialog in either case)
// TODO: point: (structure) NPC dialog can have the effect of giving the player items or spawning new enemies or starting a battle or whatever (modify dialog.txt as needed, maybe update the speaker's dlg index afterwards so they don't give the item twice!)
// TODO: point: (structure) combat screen with turn taking combat with enemies (entered via random chance or by bumping into enemies)
// TODO: point: (style) display of player and enemy stats during battle
// TODO: point: (style) transition animation in and out of combat
// TODO: point: (structure) inventory menu and getting items from chests/battle
// TODO: point: (structure) statistics menu and stat growth through battles or field events
// TODO: point: (structure) multiple party members who trail you around and act in battle

enum GameMode {
    Map,
    Battle,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let source =
        assets_manager::source::FileSystem::new("content").expect("Couldn't load resources");
    #[cfg(target_arch = "wasm32")]
    let source = assets_manager::source::Embedded::from(assets_manager::source::embed!("content"));
    let cache = assets_manager::AssetCache::with_source(source);

    let drv = frenderer::Driver::new(
        winit::window::WindowBuilder::new()
            .with_title("test")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0)),
        Some((1024, 768)),
    );

    const DT: f32 = 1.0 / 60.0;
    let mut input = Input::default();

    let mut now = frenderer::clock::Instant::now();
    let mut acc = 0.0;
    drv.run_event_loop::<(), _>(
        move |window, mut frend| {
            let game = Game::new(&mut frend, &cache);
            (window, game, frend)
        },
        move |event, target, (window, ref mut game, ref mut frend)| {
            use winit::event::{Event, WindowEvent};
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    target.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    if !frend.gpu.is_web() {
                        frend.resize_surface(size.width, size.height);
                    }
                    window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let elapsed = now.elapsed().as_secs_f32();
                    // You can add the time snapping/death spiral prevention stuff here if you want.
                    // I'm not using it here to keep the starter code small.
                    acc += elapsed;
                    now = std::time::Instant::now();
                    // While we have time to spend
                    while acc >= DT {
                        // simulate a frame
                        acc -= DT;
                        game.simulate(&input, DT);
                        input.next_frame();
                    }
                    game.render(frend);
                    frend.render();
                    window.request_redraw();
                }
                event => {
                    input.process_input_event(&event);
                }
            }
        },
    )
    .expect("event loop error");
}

impl Game {
    fn new(renderer: &mut Renderer, cache: &AssetCache) -> Self {
        let tile_handle = cache
            .load::<Png>("tilesheet")
            .expect("Couldn't load tilesheet img");
        let tile_img = tile_handle.read().0.to_rgba8();
        let tile_tex = renderer.create_array_texture(
            &[&tile_img],
            wgpu::TextureFormat::Rgba8UnormSrgb,
            tile_img.dimensions(),
            Some("tiles-sprites"),
        );
        let levels = vec![
            Level::from_str(
                &cache
                    .load::<String>("level1")
                    .expect("Couldn't access level1.txt")
                    .read(),
            ),
            Level::from_str(
                &cache
                    .load::<String>("level2")
                    .expect("Couldn't access level2.txt")
                    .read(),
            ),
        ];
        let current_level = 0;
        let dialogs = cache
            .load::<String>("dialog")
            .expect("couldn't access dialog.txt")
            .read()
            .lines()
            .map(str::to_string)
            .collect();
        // TODO: will need to parse the dialogs specially if you add yes/no or item rewards or whatever, probably into a Dialog struct instead of a string
        let camera = Camera2D {
            screen_pos: [0.0, 0.0],
            screen_size: [W as f32, H as f32],
        };
        let sprite_estimate =
            levels[current_level].sprite_count() + levels[current_level].starts().len();
        renderer.sprite_group_add(
            &tile_tex,
            vec![Transform::ZERO; sprite_estimate],
            vec![SheetRegion::ZERO; sprite_estimate],
            camera,
        );
        let player_start = *levels[current_level]
            .starts()
            .iter()
            .find(|(t, _)| *t == EntityType::Player)
            .map(|(_, ploc)| ploc)
            .expect("Start level doesn't put the player anywhere");
        // TODO initialize your game here
        let mut game = Game {
            levels,
            current_level,
            dialogs,
            mode: GameMode::Map,
            active_dialog: None,
            npcs: vec![],
            doors: vec![],
            player: player_start,
            font: frenderer::bitfont::BitFont::with_sheet_region(
                ' '..='~',
                SheetRegion::new(0, 0, 738, 0, 288, 765),
                8,
                8,
                1,
                1,
            ),
            window: frenderer::nineslice::NineSlice::with_corner_edge_center(
                frenderer::nineslice::CornerSlice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(748, 425, 16, 16).with_depth(0),
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(748, 425 + 17, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(748 + 17, 425, 16, 16).with_depth(1),
                    repeat: frenderer::nineslice::Repeat::Tile,
                },
                frenderer::nineslice::Slice {
                    w: 16.0,
                    h: 16.0,
                    region: SheetRegion::rect(748 + 17, 425 + 17, 16, 16).with_depth(2),
                    repeat: frenderer::nineslice::Repeat::Stretch,
                },
            ),
        };
        game.enter_level(player_start);
        game
    }
    fn enter_level(&mut self, player_pos: Vec2) {
        self.doors.clear();
        self.npcs.clear();
        self.player = player_pos;
        let level = &self.levels[self.current_level];
        for (etype, pos) in level.starts().iter() {
            match etype {
                EntityType::Player => {}
                EntityType::Door(rm, x, y) => {
                    self.doors.push((rm.clone(), Vec2 { x: *x, y: *y }, *pos))
                }
                EntityType::Npc(dlg) => self.npcs.push((*pos, *dlg)),
            }
        }
    }
    fn level(&self) -> &Level {
        &self.levels[self.current_level]
    }
    fn sprite_count(&self) -> usize {
        // TODO: do something different in battle mode
        self.level().sprite_count()
            + self.npcs.len()
            + self.doors.len()
            + 1
            + self
                .active_dialog
                .map(|dlg| self.dialogs[dlg].len() + self.window.sprite_count(WIND_W, WIND_H))
                .unwrap_or(0)
    }
    fn render(&mut self, frend: &mut Renderer) {
        // You could do `match self.game_mode { GameMode::Map => {...}, GameMode::Battle=> {...}}` in here

        // make this exactly as big as we need
        frend.sprite_group_resize(0, self.sprite_count());

        let sprites_used = self.level().render_into(frend, 0);
        let (sprite_posns, sprite_gfx) = frend.sprites_mut(0, sprites_used..);

        for ((npc, _dlg), (trf, uv)) in self
            .npcs
            .iter()
            .zip(sprite_posns.iter_mut().zip(sprite_gfx.iter_mut()))
        {
            *trf = Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: (npc.x * TILE_SZ as u16 + TILE_SZ as u16 / 2) as f32,
                y: (H as u16 - npc.y * TILE_SZ as u16 - TILE_SZ as u16 / 2) as f32,
                rot: 0.0,
            };
            *uv = NPC;
        }
        let sprite_posns = &mut sprite_posns[self.npcs.len()..];
        let sprite_gfx = &mut sprite_gfx[self.npcs.len()..];
        for ((_door_to, _door_to_pos, door_pos), (trf, uv)) in self
            .doors
            .iter()
            .zip(sprite_posns.iter_mut().zip(sprite_gfx.iter_mut()))
        {
            *trf = Transform {
                w: TILE_SZ as u16,
                h: TILE_SZ as u16,
                x: (door_pos.x * TILE_SZ as u16 + TILE_SZ as u16 / 2) as f32,
                y: (H as u16 - door_pos.y * TILE_SZ as u16 - TILE_SZ as u16 / 2) as f32,
                rot: 0.0,
            };
            *uv = DOOR;
        }
        let sprite_posns = &mut sprite_posns[self.doors.len()..];
        let sprite_gfx = &mut sprite_gfx[self.doors.len()..];
        sprite_posns[0] = Transform {
            w: TILE_SZ as u16,
            h: TILE_SZ as u16,
            x: (self.player.x * TILE_SZ as u16 + TILE_SZ as u16 / 2) as f32,
            y: (H as u16 - self.player.y * TILE_SZ as u16 - TILE_SZ as u16 / 2) as f32,
            rot: 0.0,
        };
        sprite_gfx[0] = PLAYER;

        let sprite_posns = &mut sprite_posns[1..];
        let sprite_gfx = &mut sprite_gfx[1..];
        // TODO: this should be extracted into a more general purpose function
        // since we want to be able to draw text into boxes at a number of different places.
        // This could turn into a call to like "draw_box_with_text_lines(...)" that returns how many sprites it uses up.
        // Or you could make a Menu struct and populate it specially for a dialog.
        if let Some(dlg) = self.active_dialog {
            let dlg = &self.dialogs[dlg];
            let mut used =
                self.window
                    .draw(sprite_posns, sprite_gfx, WIND_X, WIND_Y, WIND_W, WIND_H, 1);
            let mut sprite_posns = &mut sprite_posns[used..];
            let mut sprite_gfx = &mut sprite_gfx[used..];
            let mut y = WIND_Y;
            for line in dlg.split("\\n") {
                self.font
                    .draw_text(sprite_posns, sprite_gfx, line, [DLG_X, y], 0, 8.0);
                used += line.len();
                sprite_posns = &mut sprite_posns[line.len()..];
                sprite_gfx = &mut sprite_gfx[line.len()..];
                y -= 12.0; // line height plus a little extra
            }
        }
    }
    fn simulate(&mut self, input: &Input, _dt: f32) {
        // TODO: in battle or menu mode, this should probably move a cursor around.
        // You could consider something like "for each menu, if the menu is active, give it a chance to handle this input and if it does handle it then return from the function".

        // use input to determine how to move your character
        // move enemies on their own
        // stop all characters from walking into solid tiles (try level.get_tile(pos))
        // etc
        let dx = if input.is_key_pressed(Key::ArrowLeft) {
            -1
        } else if input.is_key_pressed(Key::ArrowRight) {
            1
        } else {
            0
        };
        // we'll continue to use "-1 towards the top of the screen" here
        let dy = if input.is_key_pressed(Key::ArrowUp) {
            -1
        } else if input.is_key_pressed(Key::ArrowDown) {
            1
        } else {
            0
        };
        let dest = Vec2 {
            x: (self.player.x as i32 + dx) as u16,
            y: (self.player.y as i32 + dy) as u16,
        };

        // dismiss dialog; this would have to change if you did yes/no in dialogs.
        // you could also use a dedicated button here instead
        if dx != 0 || dy != 0 {
            self.active_dialog = None;
        }

        let moved = dest != self.player
            && if let Some(TileData { solid: false, .. }) = self.level().get_tile(dest) {
                if let Some((_npc, dlg)) = self.npcs.iter().find(|(p, _dlg)| *p == dest) {
                    // open a dialog
                    self.active_dialog = Some(*dlg);
                    false
                } else {
                    self.player = dest;
                    true
                }
            } else {
                false
            };
        if moved {
            for (door_to, door_to_pos, door_pos) in self.doors.iter() {
                if *door_pos == self.player {
                    let dest = self
                        .levels
                        .iter()
                        .position(|l| l.name() == door_to)
                        .expect("door to invalid room {door_to}!");
                    self.current_level = dest;
                    self.enter_level(*door_to_pos);
                    break;
                }
            }
        }
    }
}

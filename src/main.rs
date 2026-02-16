// Required includes
#![no_std]
#![no_main]
// Test includes
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use agb::display::{
    Graphics, GraphicsFrame, Priority,
    object::Object,
    tile_data::TileData,
    tiled::{RegularBackground, RegularBackgroundSize, TileFormat, TileSet, VRAM_MANAGER},
};
use agb::fixnum::{Num, Rect, Vector2D, num, vec2};
use agb::input::{Button, ButtonController};
use agb::sound::mixer::{Frequency, Mixer, SoundChannel, SoundData};
use agb::{include_aseprite, include_background_gfx, include_wav};
use agb_tracker::{Track, Tracker, include_xm};
use alloc::vec::Vec;

// Background import
include_background_gfx!(
    mod background,
    BLOCKS => deduplicate "gfx/blocks.aseprite",
    NUMBERS => deduplicate "gfx/numbers.aseprite",
);

// Sprite import
include_aseprite!(
    mod sprites,
    "gfx/cursor.aseprite",
);

// Music and Sound import
static CURSOR_MOVE: SoundData = include_wav!("sfx/ball-paddle-hit.wav");
static BGM: Track = include_xm!("sfx/bgm.xm");

type Fixed = Num<i32, 8>;

pub struct PlayerCursor {
    pos: Vector2D<Fixed>,
}

impl PlayerCursor {
    pub fn new(pos: Vector2D<Fixed>) -> Self {
        Self { pos: pos }
    }

    pub fn set_pos(&mut self, pos: Vector2D<Fixed>) {
        self.pos = pos;
    }

    pub fn move_by(&mut self, pos: Vector2D<Fixed>, mixer: &mut Mixer) {
        self.pos += pos;
        if pos.x != num!(0) || pos.y != num!(0) {
            let hit_sound = SoundChannel::new(CURSOR_MOVE);
            mixer.play_sound(hit_sound);
        }
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        let sprite_pos = self.pos.round();
        Object::new(sprites::CURSOR.sprite(0))
            .set_pos(sprite_pos)
            .show(frame);
    }
}

pub enum MinefieldBlock {
    Clear,
    Block,
    Flag,
    Question,
}

pub enum MinefieldItem {
    Blank,
    Number1,
    Number2,
    Number3,
    Number4,
    Number5,
    Number6,
    Number7,
    Number8,
    Mine,
}

pub struct Minefield {
    size: Vector2D<i32>,
    mines: Vec<bool>,
    blocks: Vec<MinefieldBlock>,
}

pub struct Tile16Indices(usize, usize, usize, usize);

impl Minefield {
    pub fn new(size: Vector2D<i32>) -> Self {
        Self {
            size: size,
            mines: Vec::with_capacity((size.x * size.y) as usize),
            blocks: Vec::with_capacity((size.x * size.y) as usize),
        }
    }

    // TODO: Place mine field into a struct
    pub fn set_size(&mut self, size: Vector2D<i32>) {
        self.size = size;
        self.mines = Vec::with_capacity((size.x * size.y) as usize);
        self.blocks = Vec::with_capacity((size.x * size.y) as usize);
    }

    pub fn gen_mines(&mut self) {
        // TODO: Generate mines
    }

    fn draw_tile16(
        bg: &mut RegularBackground,
        tile_pos: Vector2D<i32>,
        tile_data: &TileData,
        tile_indices: Tile16Indices,
    ) {
        for y in 0..2 {
            for x in 0..2 {
                // Index alternates between 0/1 for even rows
                // and 2/3 for odd rows, forming a 16x16 block
                let tile_index = (x % 2 + (y % 2 * 2)) as usize;
                // TODO: Use provided tile indices

                bg.set_tile(
                    (tile_pos.x + x, tile_pos.y + y),
                    &tile_data.tiles,
                    tile_data.tile_settings[tile_index],
                );
            }
        }
    }

    fn rowcol_to_index(&self, rowcol: Vector2D<i32>) -> usize {
        (rowcol.x + rowcol.y * self.size.x) as usize
    }

    pub fn draw_minefield(&self, bg: &mut RegularBackground, tile_pos: Vector2D<i32>) {
        // Draw all the blocks
        for col in (0..self.size.y).step_by(2) {
            for row in (0..self.size.x).step_by(2) {
                let index = self.rowcol_to_index(vec2(row, col));
                // TODO: Draw all blocks (modify tile_pos)
                Self::draw_tile16(
                    bg,
                    tile_pos + vec2(row, col),
                    &background::BLOCKS,
                    Tile16Indices(1, 2, 3, 4),
                );
            }
        }
    }
}

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    // Input manager, responsible for button presses
    let mut button_controller = ButtonController::new();

    // Background
    VRAM_MANAGER.set_background_palettes(background::PALETTES);

    let mut bg = RegularBackground::new(
        Priority::P3,
        RegularBackgroundSize::Background32x32,
        TileFormat::FourBpp,
    );

    // Draw the player cursor

    // Get the graphics manager, responsible for all the graphics
    let mut gfx = gba.graphics.get();

    // Sound mixer
    let mut mixer = gba.mixer.mixer(Frequency::Hz32768);

    // Tracker for BGM
    let mut tracker = Tracker::new(&BGM);

    // Draw blank block tiles
    let mut minefield = Minefield::new(vec2(26, 16));
    minefield.draw_minefield(&mut bg, vec2(2, 2));

    // Player cursor sprite
    let mut player_cursor = PlayerCursor::new(vec2(num!(112), num!(64))); // the left paddle

    loop {
        // Read buttons
        button_controller.update();

        // Move the cursor
        player_cursor.move_by(
            vec2(
                Fixed::from(16 * button_controller.just_pressed_x_tri() as i32),
                Fixed::from(16 * button_controller.just_pressed_y_tri() as i32),
            ),
            &mut mixer,
        );

        // Prepare the frame
        let mut frame = gfx.frame();

        bg.show(&mut frame);
        player_cursor.show(&mut frame);
        tracker.step(&mut mixer);
        mixer.frame();
        frame.commit();
    }
}

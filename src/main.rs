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
    "16171a",
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
        Self { pos }
    }

    pub fn set_pos(&mut self, pos: Vector2D<Fixed>) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn move_by(&mut self, pos: Vector2D<Fixed>, mixer: &mut Mixer) -> &mut Self {
        self.pos += pos;
        if pos.x != num!(0) || pos.y != num!(0) {
            let hit_sound = SoundChannel::new(CURSOR_MOVE);
            mixer.play_sound(hit_sound);
        }
        self
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        let sprite_pos = self.pos.round();
        Object::new(sprites::CURSOR.sprite(0))
            .set_pos(sprite_pos)
            .show(frame);
    }

    pub fn collision_rect(&self) -> Rect<Fixed> {
        Rect::new(self.pos, vec2(num!(16), num!(16)))
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
    pos: Vector2D<Fixed>,
    mines: Vec<bool>,
    blocks: Vec<MinefieldBlock>,
    cursor: PlayerCursor,
}

pub struct Tile16Indices(usize, usize, usize, usize);

impl Minefield {
    /// Create a minefield with `size` (w x h) at pixel position `pos`
    pub fn new(size: Vector2D<i32>, pos: Vector2D<Fixed>) -> Self {
        Self {
            size,
            pos,
            mines: Vec::with_capacity((size.x * size.y) as usize),
            blocks: Vec::with_capacity((size.x * size.y) as usize),
            cursor: PlayerCursor::new(pos),
        }
    }

    pub fn set_size(&mut self, size: Vector2D<i32>) -> &mut Self {
        self.size = size;
        self.mines = Vec::with_capacity((size.x * size.y) as usize);
        self.blocks = Vec::with_capacity((size.x * size.y) as usize);
        self
    }

    pub fn set_pos(&mut self, bg: &mut RegularBackground, pos: Vector2D<Fixed>) -> &mut Self {
        // Move the minefield and adjust the cursor accordingly
        let prev_pos = self.pos;
        self.pos = pos;

        // Set the pos of the minefield
        let pixel_pos = pos.round();
        bg.set_scroll_pos((-pixel_pos.x, -pixel_pos.y));

        // Set the pos of the cursor
        self.cursor.set_pos(self.cursor.pos + (pos - prev_pos));
        self
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

    pub fn draw_minefield(&self, bg: &mut RegularBackground) {
        let tile_pos = vec2(0, 0);
        // Draw all the blocks
        for col in 0..self.size.y {
            for row in 0..self.size.x {
                let index = self.rowcol_to_index(vec2(row, col));
                // TODO: Draw all blocks (modify tile_pos)
                Self::draw_tile16(
                    bg,
                    tile_pos + vec2(row * 2, col * 2),
                    &background::BLOCKS,
                    Tile16Indices(1, 2, 3, 4),
                );
            }
        }

        // Scroll the background to take into account off-tile position
        let pos = self.pos.round();
        bg.set_scroll_pos((-pos.x, -pos.y));
    }

    pub fn update(
        &mut self,
        bg: &mut RegularBackground,
        button_controller: &ButtonController,
        mixer: &mut Mixer,
    ) {
        // Compute where the cursor would move to
        let maybe_move_by = vec2(
            Fixed::from(16 * button_controller.just_pressed_x_tri() as i32),
            Fixed::from(16 * button_controller.just_pressed_y_tri() as i32),
        );

        // Block the cursor from moving if it would go off of the minefield area
        let maybe_cursor_pos = self.cursor.pos + maybe_move_by;
        let cursor_size = self.cursor.collision_rect().size;

        // Early return if cursor would exit the minefield
        if maybe_cursor_pos.x < self.pos.x
            || maybe_cursor_pos.x + cursor_size.x > self.pos.x + self.size.x * 16
            || maybe_cursor_pos.y < self.pos.y
            || maybe_cursor_pos.y + cursor_size.y > self.pos.y + self.size.y * 16
        {
            return;
        }

        // Move the cursor based on controller input
        self.cursor.move_by(maybe_move_by, mixer);
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        self.cursor.show(frame);
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
    let mut minefield = Minefield::new(vec2(13, 8), vec2(num!(16), num!(16)));
    minefield.draw_minefield(&mut bg);

    loop {
        // Read buttons
        button_controller.update();

        minefield.update(&mut bg, &button_controller, &mut mixer);

        // Prepare the frame
        let mut frame = gfx.frame();

        bg.show(&mut frame);
        minefield.show(&mut frame);
        tracker.step(&mut mixer);
        mixer.frame();
        frame.commit();
    }
}

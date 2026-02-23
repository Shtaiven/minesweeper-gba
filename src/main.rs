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
    tiled::{
        RegularBackground, RegularBackgroundSize, TileFormat, TileSet, TileSetting, VRAM_MANAGER,
    },
};
use agb::fixnum::{Num, Rect, Vector2D, num, vec2};
use agb::input::{Button, ButtonController};
use agb::sound::mixer::{Frequency, Mixer, SoundChannel, SoundData};
use agb::{include_aseprite, include_background_gfx, include_wav};
use agb_tracker::{Track, Tracker, include_xm};
use alloc::vec;
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

#[derive(Clone)]
pub enum MinefieldBlock {
    Clear,
    Block,
    Flag,
    Question,
}

pub enum MinefieldItem {
    Blank,
    Number(u32),
    Mine,
}

pub struct Minefield {
    size: Vector2D<i32>,
    pos: Vector2D<Fixed>,
    mines: Vec<bool>,
    blocks: Vec<MinefieldBlock>,
    cursor: PlayerCursor,
}

pub struct Tile16Indices([usize; 4]);

impl Minefield {
    /// Create a minefield with `size` (w x h) at pixel position `pos`
    pub fn new(size: Vector2D<i32>, pos: Vector2D<Fixed>) -> Self {
        let mines = vec![false; (size.x * size.y) as usize];
        let blocks = vec![MinefieldBlock::Block; (size.x * size.y) as usize];
        Self {
            size,
            pos,
            mines,
            blocks,
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
                let tile_index = (x + (y * 2)) as usize;
                bg.set_tile(
                    (tile_pos.x + x, tile_pos.y + y),
                    &tile_data.tiles,
                    tile_data.tile_settings[tile_indices.0[tile_index]],
                );
            }
        }
    }

    fn clear_tile16(bg: &mut RegularBackground, tile_pos: Vector2D<i32>, tile_data: &TileData) {
        for y in 0..2 {
            for x in 0..2 {
                bg.set_tile(
                    (tile_pos.x + x, tile_pos.y + y),
                    &tile_data.tiles,
                    TileSetting::BLANK,
                );
            }
        }
    }

    fn tile_pos_to_index(&self, tile_pos: Vector2D<i32>) -> usize {
        (tile_pos.x + tile_pos.y * self.size.x) as usize
    }

    pub fn draw_minefield(&self, bg: &mut RegularBackground) {
        let tile_pos = vec2(0, 0);
        // Draw all the blocks
        for col in 0..self.size.y {
            for row in 0..self.size.x {
                let index = self.tile_pos_to_index(vec2(row, col));
                // TODO: Draw all blocks based on self.blocks
                Self::draw_tile16(
                    bg,
                    tile_pos + vec2(row * 2, col * 2),
                    &background::BLOCKS,
                    Tile16Indices([0, 1, 2, 3]),
                );
            }
        }

        // Scroll the background to take into account off-tile position
        let pos = self.pos.round();
        bg.set_scroll_pos((-pos.x, -pos.y));
    }

    pub fn remove_tile(
        &mut self,
        bg: &mut RegularBackground,
        tile_pos: Vector2D<i32>,
        tile_data: &TileData,
    ) {
        // Early exit if the tile position isn't within bounds
        // The size is in 16x16 tiles, but the tile_pos is in 8x8 tiles, so the size needs to be
        // multiplied by 2
        let size_8x8 = self.size;
        if tile_pos.x >= size_8x8.x || tile_pos.y >= size_8x8.y || tile_pos.x < 0 || tile_pos.y < 0
        {
            return;
        }

        let index = self.tile_pos_to_index(tile_pos);
        // Check if the tile can be cleared (i.e. not cleared or flagged status)
        match self.blocks[index] {
            MinefieldBlock::Clear | MinefieldBlock::Flag => return,
            _ => (),
        }

        // Set the clear status of the tile
        self.blocks[index] = MinefieldBlock::Clear;

        // Clear the tile
        // Multiply tile_pos by 2 because clear tile uses 8x8 coordinates
        Self::clear_tile16(bg, tile_pos * 2, tile_data);
    }

    pub fn cycle_tile_state(
        &mut self,
        bg: &mut RegularBackground,
        tile_pos: Vector2D<i32>,
        tile_data: &TileData,
    ) {
        // Early exit if the tile position isn't within bounds
        // The size is in 16x16 tiles, but the tile_pos is in 8x8 tiles, so the size needs to be
        // multiplied by 2
        let size_8x8 = self.size * 2;
        if tile_pos.x >= size_8x8.x || tile_pos.y >= size_8x8.y || tile_pos.x < 0 || tile_pos.y < 0
        {
            return;
        }

        let index = self.tile_pos_to_index(tile_pos);
        // Check if the tile can be modified (i.e. not cleared)
        let next_block_type: MinefieldBlock;
        let tile_indices: Tile16Indices;
        match self.blocks[index] {
            MinefieldBlock::Clear => return,
            MinefieldBlock::Block => {
                next_block_type = MinefieldBlock::Flag;
                tile_indices = Tile16Indices([4, 5, 6, 7]);
            }
            MinefieldBlock::Flag => {
                next_block_type = MinefieldBlock::Question;
                tile_indices = Tile16Indices([8, 9, 10, 11]);
            }
            MinefieldBlock::Question => {
                next_block_type = MinefieldBlock::Block;
                tile_indices = Tile16Indices([0, 1, 2, 3]);
            }
        }

        // Set the clear status of the tile
        self.blocks[index] = next_block_type;

        // Draw the tile
        Self::draw_tile16(bg, tile_pos * 2, tile_data, tile_indices);
    }

    fn tile_under_cursor(&self) -> Vector2D<i32> {
        let &cursor_pos = &self.cursor.pos;
        let tile_pos = (cursor_pos - self.pos).round() / 16;
        tile_pos
    }

    pub fn update(
        &mut self,
        bg: &mut RegularBackground,
        button_controller: &ButtonController,
        mixer: &mut Mixer,
    ) {
        if button_controller.is_just_pressed(Button::A) {
            self.remove_tile(bg, self.tile_under_cursor(), &background::BLOCKS);
            return;
        }

        if button_controller.is_just_pressed(Button::B) {
            self.cycle_tile_state(bg, self.tile_under_cursor(), &background::BLOCKS);
            return;
        }

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

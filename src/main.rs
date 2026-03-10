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

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum GameState {
    Play,           // normal player interaction with the game field
    GameOver(bool), // bool is for win state, true for win, false for loss
}

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

pub struct BlockIndices {
    indices: [usize; 4],
}

#[derive(PartialEq, Eq, Clone)]
pub enum MinefieldBlock {
    Clear,
    Block,
    Flag,
    Question,
}

impl MinefieldBlock {
    pub fn get_block_indices(&self) -> BlockIndices {
        use MinefieldBlock::*;
        match *self {
            Clear => BlockIndices {
                indices: [0, 1, 2, 3],
            },
            Block => BlockIndices {
                indices: [0, 1, 2, 3],
            },
            Flag => BlockIndices {
                indices: [4, 5, 6, 7],
            },
            Question => BlockIndices {
                indices: [8, 9, 10, 11],
            },
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MinefieldItem {
    Blank,
    Number(u32),
    Mine,
}

impl MinefieldItem {
    pub fn get_block_indices(&self) -> BlockIndices {
        use MinefieldItem::*;
        match *self {
            Blank => BlockIndices {
                indices: [0, 1, 2, 3],
            },
            Number(n) => {
                let offset = (n as usize - 1) << 2;
                BlockIndices {
                    indices: [0 + offset, 1 + offset, 2 + offset, 3 + offset],
                }
            }
            Mine => BlockIndices {
                indices: [32, 33, 34, 35],
            },
        }
    }
}
fn draw_block(
    bg: &mut RegularBackground,
    tile_pos: Vector2D<i32>,
    tile_data: &TileData,
    tile_indices: BlockIndices,
) {
    for y in 0..2 {
        for x in 0..2 {
            // Index alternates between 0/1 for even rows
            // and 2/3 for odd rows, forming a 16x16 block
            let tile_index = (x + (y * 2)) as usize;
            bg.set_tile(
                (tile_pos.x + x, tile_pos.y + y),
                &tile_data.tiles,
                tile_data.tile_settings[tile_indices.indices[tile_index]],
            );
        }
    }
}

fn clear_block(bg: &mut RegularBackground, tile_pos: Vector2D<i32>, tile_data: &TileData) {
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

pub struct Minefield {
    size: Vector2D<i32>,
    pos: Vector2D<Fixed>,
    mines: Vec<bool>,
    blocks: Vec<MinefieldBlock>,
    cursor: PlayerCursor,
    blocks_to_clear: Vec<Vector2D<i32>>,
}

impl Minefield {
    /// Create a minefield with block `size` (w x h) at pixel position `pos`
    pub fn new(size: Vector2D<i32>, pos: Vector2D<Fixed>) -> Self {
        let mines = vec![false; (size.x * size.y) as usize];
        let blocks = vec![MinefieldBlock::Block; (size.x * size.y) as usize];
        Self {
            size,
            pos,
            mines,
            blocks,
            cursor: PlayerCursor::new(pos),
            blocks_to_clear: vec![],
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
        // Generate mines
        for mine in &mut self.mines {
            let rand_num = agb::rng::next_i32();

            // 1/8 chance, avoids division
            *mine = rand_num.abs() < i32::MAX >> 3;
        }
    }

    fn block_pos_to_index(&self, block_pos: Vector2D<i32>) -> usize {
        (block_pos.x + block_pos.y * self.size.x) as usize
    }

    pub fn draw_minefield(&self, bg: &mut RegularBackground) {
        let tile_pos = vec2(0, 0);
        // Draw all the blocks based on what's contained in self.blocks
        for col in 0..self.size.y {
            for row in 0..self.size.x {
                let index = self.block_pos_to_index(vec2(row, col));
                draw_block(
                    bg,
                    tile_pos + vec2(row * 2, col * 2),
                    &background::BLOCKS,
                    self.blocks[index].get_block_indices(),
                );
            }
        }

        // Scroll the background to take into account off-tile position
        let pos = self.pos.round();
        bg.set_scroll_pos((-pos.x, -pos.y));
    }

    fn determine_minefield_item(&self, block_pos: &Vector2D<i32>) -> MinefieldItem {
        // First check if the item is a mine and return that if it is
        let index = self.block_pos_to_index(*block_pos);
        if self.mines[index] {
            return MinefieldItem::Mine;
        }

        // Get the 8 tiles around the current tile_pos
        let mut mine_count = 0u32;

        // Determine the number of mines
        for y_offset in -1..2 {
            for x_offset in -1..2 {
                let search_pos = vec2(block_pos.x + x_offset, block_pos.y + y_offset);
                if search_pos.x < 0
                    || search_pos.y < 0
                    || search_pos.x >= self.size.x
                    || search_pos.y >= self.size.y
                {
                    continue;
                }
                mine_count += self.mines[self.block_pos_to_index(search_pos)] as u32;
            }
        }

        // Return Number if there are any mines, otherwise blank
        if mine_count > 0 {
            return MinefieldItem::Number(mine_count);
        }
        return MinefieldItem::Blank;
    }

    pub fn remove_block(
        &mut self,
        bg: &mut RegularBackground,
        block_pos: Vector2D<i32>,
        force_remove: bool,
    ) -> MinefieldItem {
        // Early exit if the tile position isn't within bounds
        if block_pos.x >= self.size.x
            || block_pos.y >= self.size.y
            || block_pos.x < 0
            || block_pos.y < 0
        {
            return MinefieldItem::Blank;
        }

        let index = self.block_pos_to_index(block_pos);

        // Determine the item to draw
        let minefield_item = self.determine_minefield_item(&block_pos);

        // Check if the tile can be cleared (i.e. not cleared or flagged status)
        if !force_remove {
            match self.blocks[index] {
                MinefieldBlock::Clear | MinefieldBlock::Flag => {
                    return minefield_item; // early return if not
                }
                _ => (),
            }
        }

        // Set the clear status of the tile
        self.blocks[index] = MinefieldBlock::Clear;

        // Clear the tile
        // Multiply block_pos by 2 because clear tile uses tile coordinates
        clear_block(bg, block_pos * 2, &background::BLOCKS);

        // Draw the item
        if minefield_item != MinefieldItem::Blank {
            draw_block(
                bg,
                block_pos * 2,
                &background::NUMBERS,
                minefield_item.get_block_indices(),
            );
        }

        // return the item so that the caller can decide on what to do
        return minefield_item;
    }

    pub fn cycle_block_state(
        &mut self,
        bg: &mut RegularBackground,
        block_pos: Vector2D<i32>,
        tile_data: &TileData,
    ) {
        // Early exit if the tile position isn't within bounds
        if block_pos.x >= self.size.x
            || block_pos.y >= self.size.y
            || block_pos.x < 0
            || block_pos.y < 0
        {
            return;
        }

        let index = self.block_pos_to_index(block_pos);
        // Check if the tile can be modified (i.e. not cleared)
        let next_block_type: MinefieldBlock;
        match self.blocks[index] {
            MinefieldBlock::Clear => return,
            MinefieldBlock::Block => {
                next_block_type = MinefieldBlock::Flag;
            }
            MinefieldBlock::Flag => {
                next_block_type = MinefieldBlock::Question;
            }
            MinefieldBlock::Question => {
                next_block_type = MinefieldBlock::Block;
            }
        }

        // Set the clear status of the tile
        let tile_indices = next_block_type.get_block_indices();
        self.blocks[index] = next_block_type;

        // Draw the tile
        draw_block(bg, block_pos * 2, tile_data, tile_indices);
    }

    fn block_under_cursor(&self) -> Vector2D<i32> {
        let &cursor_pos = &self.cursor.pos;
        let tile_pos = (cursor_pos - self.pos).round() / 16;
        tile_pos
    }

    fn is_win_condition(&self) -> bool {
        // win condition is all non-mine blocks are cleared
        for (mine, block) in self.mines.iter().zip(self.blocks.iter()) {
            // if the block doesn't contain a mine and it is cleared
            if !mine && block != &MinefieldBlock::Clear {
                return false;
            }
        }
        return true;
    }

    fn get_surrounding_uncleared_blocks(&self, block_pos: Vector2D<i32>) -> Vec<Vector2D<i32>> {
        let mut surrounding_blocks = vec![];
        for y_offset in -1..2 {
            for x_offset in -1..2 {
                let potential_block = vec2(block_pos.x + x_offset, block_pos.y + y_offset);
                // Don't return the current block
                if x_offset == 0 && y_offset == 0 {
                    continue;
                }

                // Don't return out-of-bounds blocks
                if potential_block.x < 0
                    || potential_block.y < 0
                    || potential_block.x >= self.size.x
                    || potential_block.y >= self.size.y
                {
                    continue;
                }

                // Don't return blocks which have already been cleared
                if self.blocks[self.block_pos_to_index(potential_block)] == MinefieldBlock::Clear {
                    continue;
                }

                surrounding_blocks.push(vec2(block_pos.x + x_offset, block_pos.y + y_offset));
            }
        }
        return surrounding_blocks;
    }

    pub fn update(
        &mut self,
        bg: &mut RegularBackground,
        button_controller: &ButtonController,
        mixer: &mut Mixer,
    ) -> GameState {
        // Handle clearing blocks on the field if a blank tile was revealed
        // We return before player input since we don't want the player to be able to do anything
        // at this point
        if !self.blocks_to_clear.is_empty() {
            // Copy what's currently in the clear list and clear the blocks to clear, since we
            // Don't want to append to the vector as we modify it
            let blocks_to_clear_copy = self.blocks_to_clear.clone();
            self.blocks_to_clear.clear();

            // Remove blocks and extend blocks to clear with blocks that come up blank
            for block in blocks_to_clear_copy {
                let item = self.remove_block(bg, block, true);
                if item == MinefieldItem::Blank {
                    self.blocks_to_clear
                        .extend(self.get_surrounding_uncleared_blocks(block));
                }
            }
            return GameState::Play;
        }

        // Handle player input
        if button_controller.is_just_pressed(Button::A) {
            let block_under_cursor = self.block_under_cursor();
            if self.blocks[self.block_pos_to_index(block_under_cursor)] == MinefieldBlock::Flag {
                return GameState::Play;
            }

            let minefield_item = self.remove_block(bg, block_under_cursor, false);

            // Go to a game over screen
            if minefield_item == MinefieldItem::Mine {
                return GameState::GameOver(false);
            }

            // Go to a win screen
            if self.is_win_condition() {
                return GameState::GameOver(true);
            }

            if minefield_item == MinefieldItem::Blank {
                self.blocks_to_clear
                    .extend(self.get_surrounding_uncleared_blocks(block_under_cursor));
            }

            return GameState::Play;
        }

        if button_controller.is_just_pressed(Button::B) {
            self.cycle_block_state(bg, self.block_under_cursor(), &background::BLOCKS);
            return GameState::Play;
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
            return GameState::Play;
        }

        // Move the cursor based on controller input
        self.cursor.move_by(maybe_move_by, mixer);

        return GameState::Play;
    }

    pub fn reveal(&mut self, bg: &mut RegularBackground) {
        for col in 0..self.size.y {
            for row in 0..self.size.x {
                self.remove_block(bg, vec2(row, col), true);
            }
        }
    }

    fn reset_blocks(&mut self) {
        self.blocks.fill(MinefieldBlock::Block);
    }

    pub fn reset(&mut self, bg: &mut RegularBackground) {
        // Reset all blocks
        self.reset_blocks();

        // Regenerate mines
        self.gen_mines();

        // Draw the minefield
        self.draw_minefield(bg);
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
    minefield.reset(&mut bg);

    let mut next_game_state = GameState::Play;
    let mut prev_game_state = next_game_state;
    let mut screen_changed;

    loop {
        // Read buttons
        button_controller.update();
        screen_changed = next_game_state != prev_game_state;

        match next_game_state {
            // Update the minefield and player cursor and check what the next game screen should be
            GameState::Play => {
                next_game_state = minefield.update(&mut bg, &button_controller, &mut mixer);
            }

            // Handle game over screen
            GameState::GameOver(is_win) => {
                // Reveal all blocks if isn't win
                if prev_game_state == GameState::Play {
                    if is_win {
                        agb::println!("You win!");
                    } else {
                        agb::println!("Game over!");
                        minefield.reveal(&mut bg);
                    }
                }

                // Ask player for start input
                if button_controller.is_just_pressed(Button::START) {
                    minefield.reset(&mut bg);
                    next_game_state = GameState::Play;
                }
            }
        }

        // Prepare the frame
        let mut frame = gfx.frame();

        bg.show(&mut frame);
        if next_game_state == GameState::Play {
            minefield.show(&mut frame);
        }
        tracker.step(&mut mixer);
        mixer.frame();
        frame.commit();

        // make the random number generator harder to predict
        let _ = agb::rng::next_i32();
        if screen_changed {
            prev_game_state = next_game_state;
        }
    }
}

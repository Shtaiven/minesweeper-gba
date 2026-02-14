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
    tiled::{RegularBackground, RegularBackgroundSize, TileFormat, VRAM_MANAGER},
};
use agb::fixnum::{Num, Rect, Vector2D, num, vec2};
use agb::input::{Button, ButtonController};
use agb::sound::mixer::{Frequency, Mixer, SoundChannel, SoundData};
use agb::{include_aseprite, include_background_gfx, include_wav};
use agb_tracker::{Track, Tracker, include_xm};

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

pub enum BlockTileType {
    BLANK,
    FLAG,
    QUESTION,
}

fn draw_block_tile(bg: &mut RegularBackground, pos: Vector2D<i32>) {
    for y in 0..2 {
        for x in 0..2 {
            // Index alternates between 0/1 for even rows
            // and 2/3 for odd rows, forming a 16x16 block
            let tile_index = (x % 2 + (y % 2 * 2)) as usize;

            bg.set_tile(
                (pos.x + x, pos.y + y),
                &background::BLOCKS.tiles,
                background::BLOCKS.tile_settings[tile_index],
            );
        }
    }
}

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

    pub fn move_by(&mut self, pos: Vector2D<Fixed>) {
        self.pos += pos;
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        let sprite_pos = self.pos.round();
        Object::new(sprites::CURSOR.sprite(0))
            .set_pos(sprite_pos)
            .show(frame);
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
    for y in (2..18).step_by(2) {
        for x in (2..28).step_by(2) {
            draw_block_tile(&mut bg, Vector2D::new(x, y));
        }
    }

    // Player cursor sprite
    let mut player_cursor = PlayerCursor::new(vec2(num!(112), num!(64))); // the left paddle

    loop {
        // Read buttons
        button_controller.update();

        // Move the cursor
        player_cursor.move_by(vec2(
            Fixed::from(16 * button_controller.just_pressed_x_tri() as i32),
            Fixed::from(16 * button_controller.just_pressed_y_tri() as i32),
        ));

        // Prepare the frame
        let mut frame = gfx.frame();

        bg.show(&mut frame);
        player_cursor.show(&mut frame);
        tracker.step(&mut mixer);
        mixer.frame();
        frame.commit();
    }
}

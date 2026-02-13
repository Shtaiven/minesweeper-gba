// Required includes
#![no_std]
#![no_main]
// Test includes
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use agb::display::{
    GraphicsFrame, Priority,
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
static BALL_PADDLE_HIT: SoundData = include_wav!("sfx/ball-paddle-hit.wav");
static BGM: Track = include_xm!("sfx/bgm.xm");

type Fixed = Num<i32, 8>;


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

    // Draw a blank block tile
    for y in 0..20 {
        for x in 0..30 {
            // Index alternates between 0/1 for even rows
            // and 2/3 for odd rows, forming a 16x16 block
            let tile_index = x%2 + (y%2 * 2);

            bg.set_tile(
                (x as i32, y as i32),
                &background::BLOCKS.tiles,
                background::BLOCKS.tile_settings[tile_index],
            );
        }
    }

    // Get the graphics manager, responsible for all the graphics
    let mut gfx = gba.graphics.get();

    // Sound mixer
    let mut mixer = gba.mixer.mixer(Frequency::Hz32768);

    // Tracker for BGM
    let mut tracker = Tracker::new(&BGM);

    loop {
        // Read buttons
        button_controller.update();


        // Prepare the frame
        let mut frame = gfx.frame();

        bg.show(&mut frame);
        tracker.step(&mut mixer);
        mixer.frame();
        frame.commit();
    }
}

// Required includes
#![no_std]
#![no_main]
// Test includes
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

mod minefield;
mod types;

extern crate alloc;

use agb::{
    display::{
        Priority,
        tiled::{RegularBackground, RegularBackgroundSize, TileFormat, VRAM_MANAGER},
    },
    fixnum::{num, vec2},
    include_aseprite, include_background_gfx, include_wav,
    input::{Button, ButtonController},
    sound::mixer::{Frequency, SoundData},
};
use agb_tracker::{Track, Tracker, include_xm};
use minefield::{Minefield, MinefieldState};

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

    // Get the graphics manager, responsible for all the graphics
    let mut gfx = gba.graphics.get();

    // Sound mixer
    let mut mixer = gba.mixer.mixer(Frequency::Hz32768);

    // Tracker for BGM
    let mut tracker = Tracker::new(&BGM);

    // Draw blank block tiles
    let mut minefield = Minefield::new(
        vec2(13, 8),
        vec2(num!(16), num!(16)),
        &background::BLOCKS,
        &background::NUMBERS,
        sprites::CURSOR.sprite(0),
        &CURSOR_MOVE,
    );
    minefield.reset(&mut bg);

    let mut next_game_state = MinefieldState::Play;
    let mut prev_game_state = next_game_state;
    let mut screen_changed;

    loop {
        // Read buttons
        button_controller.update();
        screen_changed = next_game_state != prev_game_state;

        match next_game_state {
            // Update the minefield and player cursor and check what the next game screen should be
            MinefieldState::Play => {
                next_game_state = minefield.update(&mut bg, &button_controller, &mut mixer);
            }

            // Handle game over screen
            MinefieldState::GameOver(is_win) => {
                // Reveal all blocks if isn't win
                if prev_game_state == MinefieldState::Play {
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
                    next_game_state = MinefieldState::Play;
                }
            }
        }

        // Prepare the frame
        let mut frame = gfx.frame();

        bg.show(&mut frame);
        if next_game_state == MinefieldState::Play {
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

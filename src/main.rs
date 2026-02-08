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
    PLAY_FIELD => deduplicate "gfx/background.aseprite",
    SCORE => deduplicate "gfx/player-health.aseprite",
);

// Sprite import
include_aseprite!(
    mod sprites,
    "gfx/sprites.aseprite",
    "gfx/cpu-health.aseprite",
);

// Music and Sound import
static BALL_PADDLE_HIT: SoundData = include_wav!("sfx/ball-paddle-hit.wav");
static BGM: Track = include_xm!("sfx/bgm.xm");

type Fixed = Num<i32, 8>;

pub struct Paddle {
    pos: Vector2D<Fixed>,
    health: i32,
    hflip: bool,
}

impl Paddle {
    pub fn new(pos: Vector2D<Fixed>, hflip: bool) -> Self {
        Self {
            pos: pos,
            health: 3,
            hflip: hflip,
        }
    }

    pub fn set_pos(&mut self, pos: Vector2D<Fixed>) {
        self.pos = pos;
    }

    pub fn set_hflip(&mut self, hflip: bool) {
        self.hflip = hflip;
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        let sprite_pos = self.pos.round();

        Object::new(sprites::PADDLE_END.sprite(0))
            .set_pos(sprite_pos)
            .set_hflip(self.hflip)
            .show(frame);
        Object::new(sprites::PADDLE_MID.sprite(0))
            .set_pos(sprite_pos + vec2(0, 16))
            .set_hflip(self.hflip)
            .show(frame);
        Object::new(sprites::PADDLE_END.sprite(0))
            .set_pos(sprite_pos + vec2(0, 32))
            .set_vflip(true)
            .set_hflip(self.hflip)
            .show(frame);
    }

    pub fn move_by(&mut self, y: Fixed) {
        // we now need to cast the 0 to a Fixed which you can do with
        // `Fixed::from(0)` or `0.into()`. But the preferred one is the `num!` macro
        // which we imported above.
        self.pos += vec2(num!(0), y);
    }

    pub fn collision_rect(&self) -> Rect<Fixed> {
        // Same idea here with creating a fixed point rectangle
        Rect::new(self.pos, vec2(num!(16), num!(16 * 3)))
    }

    pub fn health(&self) -> i32 {
        self.health
    }

    pub fn decrement_health(&mut self) {
        self.health -= 1;
    }
}

pub struct Ball {
    pos: Vector2D<Fixed>,
    velocity: Vector2D<Fixed>,
}

impl Ball {
    pub fn new(pos: Vector2D<Fixed>, velocity: Vector2D<Fixed>) -> Self {
        Self { pos, velocity }
    }

    pub fn update(&mut self, paddle_a: &mut Paddle, paddle_b: &mut Paddle, mixer: &mut Mixer) {
        // Speculatively move the ball, we'll update the velocity if this causes it to intersect with either the
        // edge of the map or a paddle.
        let potential_ball_pos = self.pos + self.velocity;

        let ball_rect = Rect::new(potential_ball_pos, vec2(num!(16), num!(16)));
        if paddle_a.collision_rect().touches(ball_rect) {
            self.velocity.x = self.velocity.x.abs();
            let y_difference = (ball_rect.centre().y - paddle_a.collision_rect().centre().y) / 32;
            self.velocity.y += y_difference;
            play_hit(mixer);
        }

        if paddle_b.collision_rect().touches(ball_rect) {
            self.velocity.x = -self.velocity.x.abs();
            let y_difference = (ball_rect.centre().y - paddle_b.collision_rect().centre().y) / 32;
            self.velocity.y += y_difference;
            play_hit(mixer);
        }

        // We check if the ball reaches the edge of the screen and reverse it's direction
        // We also decrement health of the appropriate player
        if potential_ball_pos.x <= num!(0) {
            self.velocity.x *= -1;
            paddle_a.decrement_health();
        } else if potential_ball_pos.x >= num!(agb::display::WIDTH - 16) {
            self.velocity.x *= -1;
            paddle_b.decrement_health();
        }

        if potential_ball_pos.y <= num!(0)
            || potential_ball_pos.y >= num!(agb::display::HEIGHT - 16)
        {
            self.velocity.y *= num!(-1);
        }

        self.pos += self.velocity;
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        let sprite_pos = self.pos.round();

        Object::new(sprites::BALL.sprite(0))
            .set_pos(sprite_pos)
            .set_priority(Priority::P1)
            .show(frame);
    }
}

fn play_hit(mixer: &mut Mixer) {
    let hit_sound = SoundChannel::new(BALL_PADDLE_HIT);
    mixer.play_sound(hit_sound);
}

fn show_cpu_health(paddle: &Paddle, frame: &mut GraphicsFrame) {
    // The text CPU: ends at exactly the edge of the sprite (which the player text doesn't).
    // so we add a 3 pixel gap between the text and the start of the hearts to make it look a bit nicer.
    const TEXT_HEART_GAP: i32 = 3;

    // The top left of the CPU health. The text is 2 tiles wide and the hearts are 3.
    // We also offset the y value by 4 pixels to keep it from the edge of the screen.
    let top_left = vec2(agb::display::WIDTH - 4 - (2 + 3) * 8 - TEXT_HEART_GAP, 4);

    // Display the text `CPU:`
    Object::new(sprites::CPU.sprite(0))
        .set_pos(top_left)
        .show(frame);
    Object::new(sprites::CPU.sprite(1))
        .set_pos(top_left + vec2(8, 0))
        .show(frame);

    // For each heart frame, show that too
    for i in 0..3 {
        let heart_frame = if i < paddle.health() { 0 } else { 1 };

        Object::new(sprites::HEART.sprite(heart_frame))
            .set_pos(top_left + vec2(16 + i * 8 + TEXT_HEART_GAP, 0))
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

    bg.fill_with(&background::PLAY_FIELD);

    let mut player_health_background = RegularBackground::new(
        Priority::P0,
        RegularBackgroundSize::Background32x32,
        TileFormat::FourBpp,
    );

    for i in 0..4 {
        player_health_background.set_tile(
            (i, 0),
            &background::SCORE.tiles,
            background::SCORE.tile_settings[i as usize],
        );
    }
    player_health_background.set_scroll_pos((-4, -4));

    // Get the graphics manager, responsible for all the graphics
    let mut gfx = gba.graphics.get();

    // Sound mixer
    let mut mixer = gba.mixer.mixer(Frequency::Hz32768);

    // Tracker for BGM
    let mut tracker = Tracker::new(&BGM);

    // Ball sprite
    let mut ball = Ball::new(vec2(num!(50), num!(50)), vec2(num!(1), num!(0.5)));

    // Paddle sprites
    let mut paddle_a = Paddle::new(vec2(num!(8), num!(8)), false); // the left paddle
    let mut paddle_b = Paddle::new(vec2(num!(240 - 16 - 8), num!(8)), true); // the right paddle

    loop {
        // Read buttons
        button_controller.update();

        // Move the player's paddle
        let mut paddle_speed_mod = 1;
        if button_controller.is_pressed(Button::A) {
            paddle_speed_mod = 2;
        }

        // Paddle a movement
        paddle_a.move_by(Fixed::from(
            paddle_speed_mod * (button_controller.y_tri() as i32),
        ));

        if paddle_a.collision_rect().position.y < num!(0) {
            paddle_a.set_pos(vec2(num!(8), num!(0)));
        }

        if paddle_a.collision_rect().position.y > num!(agb::display::HEIGHT - 48) {
            paddle_a.set_pos(vec2(num!(8), num!(agb::display::HEIGHT - 48)));
        }

        // Paddle b movement
        if ball.pos.y < paddle_b.pos.y {
            paddle_b.move_by(num!(-1));
        }

        if ball.pos.y + num!(16) > paddle_b.pos.y + num!(48) {
            paddle_b.move_by(num!(1));
        }

        if paddle_b.collision_rect().position.y < num!(0) {
            paddle_b.set_pos(vec2(num!(agb::display::WIDTH - 16 - 8), num!(0)));
        }

        if paddle_b.collision_rect().position.y > num!(agb::display::HEIGHT - 48) {
            paddle_b.set_pos(vec2(
                num!(agb::display::WIDTH - 16 - 8),
                num!(agb::display::HEIGHT - 48),
            ));
        }

        ball.update(&mut paddle_a, &mut paddle_b, &mut mixer);

        // Draw player health
        for i in 0..3 {
            // Tile 4 is filled heart, tile 5 is empty heart
            let tile_index = if i < paddle_a.health() { 4 } else { 5 };
            player_health_background.set_tile(
                (i + 4, 0),
                &background::SCORE.tiles,
                background::SCORE.tile_settings[tile_index],
            );
        }

        // Prepare the frame
        let mut frame = gfx.frame();
        ball.show(&mut frame);
        paddle_a.show(&mut frame);
        paddle_b.show(&mut frame);

        bg.show(&mut frame);
        player_health_background.show(&mut frame); // Test showing health with the background
        show_cpu_health(&paddle_b, &mut frame); // Test showing health with sprites
        tracker.step(&mut mixer);
        mixer.frame();
        frame.commit();
    }
}

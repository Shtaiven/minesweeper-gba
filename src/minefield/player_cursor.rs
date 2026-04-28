use agb::{
    display::{
        GraphicsFrame,
        object::{Object, Sprite},
    },
    fixnum::{Rect, Vector2D, num, vec2},
    sound::mixer::{Mixer, SoundChannel, SoundData},
};

use crate::types::Fixed;

pub struct PlayerCursor {
    pub pos: Vector2D<Fixed>,
    sprite_cursor: &'static Sprite,
    sound_cursor_move: &'static SoundData,
}

impl PlayerCursor {
    pub fn new(
        pos: Vector2D<Fixed>,
        sprite_cursor: &'static Sprite,
        sound_cursor_move: &'static SoundData,
    ) -> Self {
        Self {
            pos,
            sprite_cursor,
            sound_cursor_move,
        }
    }

    pub fn set_pos(&mut self, pos: Vector2D<Fixed>) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn move_by(&mut self, pos: Vector2D<Fixed>, mixer: &mut Mixer) -> &mut Self {
        self.pos += pos;
        if pos.x != num!(0) || pos.y != num!(0) {
            let hit_sound = SoundChannel::new(*self.sound_cursor_move);
            mixer.play_sound(hit_sound);
        }
        self
    }

    pub fn show(&self, frame: &mut GraphicsFrame) {
        let sprite_pos = self.pos.round();
        Object::new(self.sprite_cursor)
            .set_pos(sprite_pos)
            .show(frame);
    }

    pub fn collision_rect(&self) -> Rect<Fixed> {
        Rect::new(self.pos, vec2(num!(16), num!(16)))
    }
}

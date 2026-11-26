//! Custom animated desktop companions (`Cat`, `Chiba` Dog, `Sheep`) for Nephron.

pub mod atlas;
pub mod widget;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PetSpecies {
    Cat,
    Chiba,
    Sheep,
}

impl PetSpecies {
    pub const ALL: [Self; 3] = [Self::Cat, Self::Chiba, Self::Sheep];

    pub fn name(self) -> &'static str {
        match self {
            Self::Cat => "Cat",
            Self::Chiba => "Chiba Dog",
            Self::Sheep => "Sheep",
        }
    }

    pub fn from_arg(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "cat" => Self::Cat,
            "sheep" => Self::Sheep,
            _ => Self::Chiba,
        }
    }

    pub fn greet(self) -> &'static str {
        match self {
            Self::Cat => "Mrrp. Ready to help.",
            Self::Chiba => "Woof! Ready to build!",
            Self::Sheep => "Baa~ Cozy & ready.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PetMood {
    Idle,
    Think,
    Talk,
    Scan,
}

pub struct PetCompanion {
    pub species: PetSpecies,
    pub mood: PetMood,
    pub frame: usize,
    pub anim_elapsed: f32,
    pub min_mood_ttl: f32,
    pub bubble: Option<String>,
    pub bubble_ttl: f32,
    pub show_input: bool,
}

impl Default for PetCompanion {
    fn default() -> Self {
        Self {
            species: PetSpecies::Chiba,
            mood: PetMood::Idle,
            frame: 0,
            anim_elapsed: 0.0,
            min_mood_ttl: 0.0,
            bubble: Some(PetSpecies::Chiba.greet().into()),
            bubble_ttl: 3.5,
            show_input: false,
        }
    }
}

impl PetCompanion {
    pub fn tick(&mut self, dt: f32) {
        self.anim_elapsed += dt;
        if self.anim_elapsed >= 0.15 {
            self.anim_elapsed = 0.0;
            let count = atlas::frames_for(self.species, self.mood).len().max(1);
            self.frame = (self.frame + 1) % count;
        }
        if self.min_mood_ttl > 0.0 {
            self.min_mood_ttl = (self.min_mood_ttl - dt).max(0.0);
        }
        if self.bubble_ttl > 0.0 {
            self.bubble_ttl = (self.bubble_ttl - dt).max(0.0);
            if self.bubble_ttl == 0.0 {
                self.bubble = None;
                if self.mood == PetMood::Talk && self.min_mood_ttl == 0.0 {
                    self.set_mood(PetMood::Idle);
                }
            }
        }
    }

    pub fn set_mood(&mut self, mood: PetMood) {
        if self.mood != mood {
            self.mood = mood;
            self.frame = 0;
            self.anim_elapsed = 0.0;
        }
    }

    pub fn say(&mut self, text: impl Into<String>, ttl_secs: f32) {
        self.bubble = Some(text.into());
        self.bubble_ttl = ttl_secs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_frames_and_moods() {
        let mut pet = PetCompanion::default();
        pet.set_mood(PetMood::Scan);
        assert_eq!(pet.mood, PetMood::Scan);
        pet.tick(0.2);
        assert_eq!(pet.frame, 1);
    }
}

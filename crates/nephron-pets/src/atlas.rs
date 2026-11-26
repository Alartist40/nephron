//! Embedded custom PNG sprite frames for Cat, Chiba Dog, and Sheep.

use crate::{PetMood, PetSpecies};
use eframe::egui::ColorImage;

const CAT_IDLE: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Cat_idle1.png"),
    include_bytes!("../../../assets/sprites/Cat_idle2.png"),
    include_bytes!("../../../assets/sprites/Cat_idle3.png"),
    include_bytes!("../../../assets/sprites/Cat_idle4.png"),
    include_bytes!("../../../assets/sprites/Cat_idle5.png"),
];
const CAT_THINK: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Cat_think1.png"),
    include_bytes!("../../../assets/sprites/Cat_think2.png"),
    include_bytes!("../../../assets/sprites/Cat_think3.png"),
    include_bytes!("../../../assets/sprites/Cat_think4.png"),
    include_bytes!("../../../assets/sprites/Cat_think5.png"),
];
const CAT_TALK: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Cat_talk1.png"),
    include_bytes!("../../../assets/sprites/Cat_talk2.png"),
    include_bytes!("../../../assets/sprites/Cat_talk3.png"),
    include_bytes!("../../../assets/sprites/Cat_talk4.png"),
    include_bytes!("../../../assets/sprites/Cat_talk5.png"),
];
const CAT_SCAN: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Cat_scan1.png"),
    include_bytes!("../../../assets/sprites/Cat_scan2.png"),
    include_bytes!("../../../assets/sprites/Cat_scan3.png"),
    include_bytes!("../../../assets/sprites/Cat_scan4.png"),
    include_bytes!("../../../assets/sprites/Cat_scan5.png"),
];

const CHIBA_IDLE: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Chiba_idle1.png"),
    include_bytes!("../../../assets/sprites/Chiba_idle2.png"),
    include_bytes!("../../../assets/sprites/Chiba_idle3.png"),
    include_bytes!("../../../assets/sprites/Chiba_idle4.png"),
    include_bytes!("../../../assets/sprites/Chiba_idle5.png"),
];
const CHIBA_THINK: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Chiba_think1.png"),
    include_bytes!("../../../assets/sprites/Chiba_think2.png"),
    include_bytes!("../../../assets/sprites/Chiba_think3.png"),
    include_bytes!("../../../assets/sprites/Chiba_think4.png"),
    include_bytes!("../../../assets/sprites/Chiba_think5.png"),
];
const CHIBA_TALK: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Chiba_talk1.png"),
    include_bytes!("../../../assets/sprites/Chiba_talk2.png"),
    include_bytes!("../../../assets/sprites/Chiba_talk3.png"),
    include_bytes!("../../../assets/sprites/Chiba_talk4.png"),
    include_bytes!("../../../assets/sprites/Chiba_talk5.png"),
];
const CHIBA_SCAN: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Chiba_scan1.png"),
    include_bytes!("../../../assets/sprites/Chiba_scan2.png"),
    include_bytes!("../../../assets/sprites/Chiba_scan3.png"),
    include_bytes!("../../../assets/sprites/Chiba_scan4.png"),
    include_bytes!("../../../assets/sprites/Chiba_scan5.png"),
];

const SHEEP_IDLE: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Sheep_idle1.png"),
    include_bytes!("../../../assets/sprites/Sheep_idle2.png"),
    include_bytes!("../../../assets/sprites/Sheep_idle3.png"),
    include_bytes!("../../../assets/sprites/Sheep_idle4.png"),
    include_bytes!("../../../assets/sprites/Sheep_idle5.png"),
];
const SHEEP_THINK: [&[u8]; 4] = [
    include_bytes!("../../../assets/sprites/Sheep_think1.png"),
    include_bytes!("../../../assets/sprites/Sheep_think2.png"),
    include_bytes!("../../../assets/sprites/Sheep_think3.png"),
    include_bytes!("../../../assets/sprites/Sheep_think4.png"),
];
const SHEEP_TALK: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Sheep_talk1.png"),
    include_bytes!("../../../assets/sprites/Sheep_talk2.png"),
    include_bytes!("../../../assets/sprites/Sheep_talk3.png"),
    include_bytes!("../../../assets/sprites/Sheep_talk4.png"),
    include_bytes!("../../../assets/sprites/Sheep_talk5.png"),
];
const SHEEP_SCAN: [&[u8]; 5] = [
    include_bytes!("../../../assets/sprites/Sheep_scan1.png"),
    include_bytes!("../../../assets/sprites/Sheep_scan2.png"),
    include_bytes!("../../../assets/sprites/Sheep_scan3.png"),
    include_bytes!("../../../assets/sprites/Sheep_scan4.png"),
    include_bytes!("../../../assets/sprites/Sheep_scan5.png"),
];

pub fn frames_for(species: PetSpecies, mood: PetMood) -> &'static [&'static [u8]] {
    match (species, mood) {
        (PetSpecies::Cat, PetMood::Idle) => &CAT_IDLE,
        (PetSpecies::Cat, PetMood::Think) => &CAT_THINK,
        (PetSpecies::Cat, PetMood::Talk) => &CAT_TALK,
        (PetSpecies::Cat, PetMood::Scan) => &CAT_SCAN,
        (PetSpecies::Chiba, PetMood::Idle) => &CHIBA_IDLE,
        (PetSpecies::Chiba, PetMood::Think) => &CHIBA_THINK,
        (PetSpecies::Chiba, PetMood::Talk) => &CHIBA_TALK,
        (PetSpecies::Chiba, PetMood::Scan) => &CHIBA_SCAN,
        (PetSpecies::Sheep, PetMood::Idle) => &SHEEP_IDLE,
        (PetSpecies::Sheep, PetMood::Think) => &SHEEP_THINK,
        (PetSpecies::Sheep, PetMood::Talk) => &SHEEP_TALK,
        (PetSpecies::Sheep, PetMood::Scan) => &SHEEP_SCAN,
    }
}

pub fn decode_frame(species: PetSpecies, mood: PetMood, frame: usize) -> ColorImage {
    let list = frames_for(species, mood);
    let bytes = list[frame % list.len()];
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("valid embedded pet png")
        .to_rgba8();
    let (w, h) = img.dimensions();
    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_species_and_moods() {
        for sp in PetSpecies::ALL {
            for mood in [PetMood::Idle, PetMood::Think, PetMood::Talk, PetMood::Scan] {
                let img = decode_frame(sp, mood, 0);
                assert!(img.size[0] > 0 && img.size[1] > 0);
            }
        }
    }
}

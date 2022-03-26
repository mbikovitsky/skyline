use std::ops::RangeInclusive;

use rand::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Building {
    height: u32,
    width: u32,
    // TODO: windows
}

#[derive(Debug, Clone, Copy)]
pub enum Pixel {
    Background,
    Border,
}

impl Building {
    pub fn new(height: u32, width: u32) -> Self {
        Self { height, width }
    }

    pub fn iter_columns(&self) -> impl Iterator<Item = Vec<Pixel>> {
        let height = self.height;
        let width = self.width;

        (0..width).map(move |col| {
            if height == 0 {
                return vec![];
            }
            
            if col == 0 || col == width - 1 {
                vec![Pixel::Border; height.try_into().unwrap()]
            } else {
                let mut cells = vec![Pixel::Background; height.try_into().unwrap()];
                cells[0] = Pixel::Border;
                cells
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct RandomBuildingGenerator {
    height_range: RangeInclusive<u32>,
    width_range: RangeInclusive<u32>,
}

impl RandomBuildingGenerator {
    pub fn new(height_range: RangeInclusive<u32>, width_range: RangeInclusive<u32>) -> Self {
        assert!(!height_range.is_empty());
        assert!(!width_range.is_empty());

        Self {
            height_range,
            width_range,
        }
    }
}

impl Iterator for RandomBuildingGenerator {
    type Item = Building;

    fn next(&mut self) -> Option<Self::Item> {
        let height = thread_rng().gen_range(self.height_range.clone());
        let width = thread_rng().gen_range(self.width_range.clone());
        Some(Building::new(height, width))
    }
}

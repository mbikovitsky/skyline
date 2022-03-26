use std::{iter, ops::Range};

use itertools::Itertools;
use rand::prelude::*;

#[derive(Debug, Clone, Copy)]
struct Building {
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

#[derive(Debug)]
struct RandomBuildingGenerator {
    height_range: Range<u32>,
    width_range: Range<u32>,
    previous_height: u32,
}

impl RandomBuildingGenerator {
    pub fn new(height_range: Range<u32>, width_range: Range<u32>) -> Self {
        assert!(!height_range.is_empty());
        assert!(height_range.end - height_range.start > 1);
        assert!(!width_range.is_empty());

        Self {
            height_range,
            width_range,
            previous_height: 0,
        }
    }
}

impl Iterator for RandomBuildingGenerator {
    type Item = Building;

    fn next(&mut self) -> Option<Self::Item> {
        let height = loop {
            let height = thread_rng().gen_range(self.height_range.clone());
            if height != self.previous_height {
                self.previous_height = height;
                break height;
            }
        };
        let width = thread_rng().gen_range(self.width_range.clone());
        Some(Building::new(height, width))
    }
}

pub fn skyline(
    height_range: Range<u32>,
    width_range: Range<u32>,
) -> impl Iterator<Item = Vec<Pixel>> {
    iter::once(vec![])
        .chain(
            RandomBuildingGenerator::new(height_range, width_range)
                .map(|building| building.iter_columns())
                .flatten(),
        )
        .tuple_windows()
        .filter_map(|(previous, current, next)| {
            if current.len() >= previous.len() {
                if current.len() >= next.len() {
                    Some(current)
                } else {
                    None
                }
            } else {
                None
            }
        })
}

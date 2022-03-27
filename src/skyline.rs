use std::{iter, ops::Range};

use itertools::Itertools;
use rand::prelude::*;

use crate::util::sample_poisson_disc_2d;

#[derive(Debug, Clone)]
struct Building {
    height: u32,
    width: u32,
    windows: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Copy)]
pub enum Pixel {
    Background,
    Border,
    Window,
}

impl Building {
    pub fn new(height: u32, width: u32, windows: Vec<(u32, u32)>) -> Self {
        Self {
            height,
            width,
            windows,
        }
    }

    pub fn iter_columns(self) -> impl Iterator<Item = Vec<Pixel>> {
        (0..self.width).map(move |col| {
            if self.height == 0 {
                return vec![];
            }

            let mut pixels = if col == 0 || col == self.width - 1 {
                vec![Pixel::Border; self.height.try_into().unwrap()]
            } else {
                let mut cells = vec![Pixel::Background; self.height.try_into().unwrap()];
                cells[0] = Pixel::Border;
                cells
            };

            for &(x, y) in self.windows.iter() {
                if x != col {
                    continue;
                }

                let y: usize = y.try_into().unwrap();
                pixels[y] = Pixel::Window;
            }

            pixels
        })
    }
}

#[derive(Debug)]
struct RandomBuildingGenerator {
    height_range: Range<u32>,
    width_range: Range<u32>,
    max_windows: usize,
    min_window_distance: u32,
    previous_height: u32,
}

impl RandomBuildingGenerator {
    pub fn new(
        height_range: Range<u32>,
        width_range: Range<u32>,
        max_windows: usize,
        min_window_distance: u32,
    ) -> Self {
        assert!(!height_range.is_empty());
        assert!(height_range.end - height_range.start > 1);
        assert!(!width_range.is_empty());

        Self {
            height_range,
            width_range,
            max_windows,
            min_window_distance,
            previous_height: 0,
        }
    }
}

impl RandomBuildingGenerator {
    fn gen_windows(&self, width: u32, height: u32) -> Vec<(u32, u32)> {
        if width < 5 || height < 4 {
            return vec![];
        }

        sample_poisson_disc_2d(
            &mut thread_rng(),
            self.min_window_distance,
            width - 4,
            height - 3,
        )
        .choose_multiple(&mut thread_rng(), self.max_windows)
        .map(|&(x, y)| (x + 2, y + 2))
        .collect()
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

        Some(Building::new(
            height,
            width,
            self.gen_windows(width, height),
        ))
    }
}

pub fn skyline(
    height_range: Range<u32>,
    width_range: Range<u32>,
    max_windows: usize,
    min_window_distance: u32,
) -> impl Iterator<Item = Vec<Pixel>> {
    iter::once(vec![])
        .chain(
            RandomBuildingGenerator::new(
                height_range,
                width_range,
                max_windows,
                min_window_distance,
            )
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

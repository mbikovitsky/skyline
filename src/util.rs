use std::{error::Error, f64::consts::PI};

use itertools::iproduct;
use rand::{distributions::Uniform, prelude::*};

const MAX_TEST_SAMPLES: usize = 30;

pub trait StringErr<T> {
    fn string_err(self) -> Result<T, String>;
}

impl<T, E: Error> StringErr<T> for Result<T, E> {
    fn string_err(self) -> Result<T, String> {
        self.map_err(|err| format!("{}", err))
    }
}

/// Performs Poisson Disc Sampling from a given 2D domain.
///
/// Arguments:
/// * `rng` - RNG to use for sampling.
/// * `min_distance` - Minimum distance between any 2 samples.
/// * `width` - Width of the sampling domain.
/// * `height` - Height of the sampling domain.
pub fn sample_poisson_disc_2d<R: Rng + ?Sized>(
    rng: &mut R,
    min_distance: u32,
    width: u32,
    height: u32,
) -> Vec<(u32, u32)> {
    assert!(min_distance <= i32::MAX as u32 / 2);
    assert!(width <= i32::MAX as u32);
    assert!(height <= i32::MAX as u32);

    // https://www.jasondavies.com/poisson-disc/
    // https://www.cs.ubc.ca/~rbridson/docs/bridson-siggraph07-poissondisk.pdf

    let mut samples = vec![];
    let mut active_list = vec![];

    samples.push((rng.gen_range(0..width), rng.gen_range(0..height)));
    active_list.push(0);

    let angles = Uniform::new(0.0, 2.0 * PI);
    let radii = Uniform::new_inclusive(min_distance as f64, 2.0 * min_distance as f64);

    while !active_list.is_empty() {
        let center_index = rng.gen_range(0..active_list.len());
        let center = active_list[center_index];

        let mut found = false;
        for _ in 0..MAX_TEST_SAMPLES {
            let angle = rng.sample(angles);
            let radius = rng.sample(radii);

            let x = (radius * angle.cos()) as i32;
            let y = (radius * angle.sin()) as i32;

            let x = match (samples[center].0 as i32).checked_add(x) {
                Some(x) => x,
                None => continue,
            };
            let y = match (samples[center].1 as i32).checked_add(y) {
                Some(y) => y,
                None => continue,
            };

            if x < 0 || y < 0 {
                continue;
            }

            let x = x as u32;
            let y = y as u32;

            if x >= width || y >= height {
                continue;
            }

            let sample_is_far_enough = samples.iter().all(|&sample| {
                let distance = ((sample.0 as f64 - x as f64).powi(2)
                    + (sample.1 as f64 - y as f64).powi(2))
                .sqrt();
                distance >= min_distance as f64
            });

            if sample_is_far_enough {
                active_list.push(samples.len());
                samples.push((x, y));
                found = true;
                break;
            }
        }
        if !found {
            active_list.swap_remove(center_index);
        }
    }

    samples
}

/// Generates the coordinates of all points within a circle of a given `radius`
/// and centered at `center`.
pub fn filled_circle(
    center: (i32, i32),
    radius: u32,
) -> impl Iterator<Item = (i32, i32)> {
    let (center_x, center_y) = center;

    assert!(radius as f64 <= (i32::MAX as f64 / 2.0).sqrt());
    let radius = radius as i32;

    assert!(center_x <= i32::MAX - radius as i32);
    assert!(center_x >= i32::MIN + radius as i32);
    assert!(center_y <= i32::MAX - radius as i32);
    assert!(center_y >= i32::MIN + radius as i32);

    iproduct!(-radius..=radius, -radius..=radius)
        .filter(move |(x, y)| x * x + y * y < radius * radius)
        .map(move |(x, y)| (x + center_x, y + center_y))
}

mod skyline;
mod util;

use std::{
    fmt::Display,
    ops::Range,
    str::FromStr,
    time::{Duration, Instant},
};

use clap::{crate_name, Parser};
use lazy_static::lazy_static;
use rand::prelude::*;
use regex::Regex;
use sdl2::{
    event::Event,
    pixels::{Color, PixelFormatEnum},
    rect::{Point, Rect},
    render::{BlendMode, Canvas, Texture, TextureCreator},
    surface::Surface,
    sys::SDL_UpperBlit,
};

use crate::{
    skyline::{skyline, Pixel},
    util::{filled_circle, sample_poisson_disc_2d, StringErr},
};

const HEIGHT_RANGE: Range<u32> = 5..51;
const WIDTH_RANGE: Range<u32> = 5..11;
const CANVAS_WIDTH: u32 = 128;
const CANVAS_HEIGHT: u32 = 96;
const MOON_CENTER: (i32, i32) = (20, 20);

const TRANSPARENT: Color = Color::RGBA(0, 0, 0, 0);

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// Frames per second for the animation.
    #[clap(short, long, default_value_t = 30)]
    fps: u32,

    /// Maximum number of windows to generate for each building.
    #[clap(short, long, default_value_t = 5)]
    windows: usize,

    /// Minimum distance between windows.
    #[clap(short = 'd', long, default_value_t = 2)]
    window_distance: u32,

    /// Number of stars to generate.
    #[clap(short, long, default_value_t = 20)]
    stars: usize,

    /// Minimum distance between stars.
    #[clap(short = 'D', long, default_value_t = 5)]
    star_distance: u32,

    /// Radius of the generated moon.
    #[clap(short, long, default_value_t = 12)]
    moon_radius: u32,

    /// Color of the sky.
    #[clap(short = 'S', long, default_value_t = ArgColor { r: 63, g: 63, b: 116 })]
    sky_color: ArgColor,

    /// Color of the building borders.
    #[clap(short, long, default_value_t = ArgColor { r: 0, g: 0, b: 0 })]
    border_color: ArgColor,

    /// Color of the building background.
    #[clap(short = 'B', long, default_value_t = ArgColor { r: 50, g: 60, b: 57 })]
    background_color: ArgColor,

    /// Color of the stars.
    #[clap(short = 'T', long, default_value_t = ArgColor { r: 255, g: 255, b: 255 })]
    star_color: ArgColor,

    /// Color of the windows.
    #[clap(short = 'W', long, default_value_t = ArgColor { r: 251, g: 242, b: 54 })]
    window_color: ArgColor,
}

#[derive(Debug, Clone, Copy)]
struct ArgColor {
    r: u8,
    g: u8,
    b: u8,
}

impl Display for ArgColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl From<ArgColor> for Color {
    fn from(color: ArgColor) -> Self {
        Self::RGB(color.r, color.g, color.b)
    }
}

impl FromStr for ArgColor {
    type Err = &'static str;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new("#(?P<r>[0-9a-f]{2})(?P<g>[0-9a-f]{2})(?P<b>[0-9a-f]{2})").unwrap();
        }

        let captures = RE.captures(string).ok_or("Invalid hex color value")?;

        let r = u8::from_str_radix(captures.name("r").unwrap().as_str(), 16).unwrap();
        let g = u8::from_str_radix(captures.name("g").unwrap().as_str(), 16).unwrap();
        let b = u8::from_str_radix(captures.name("b").unwrap().as_str(), 16).unwrap();

        Ok(ArgColor { r, g, b })
    }
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    // 0 means nearest-neighbour
    // https://wiki.libsdl.org/SDL_HINT_RENDER_SCALE_QUALITY
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");

    let window = video_subsystem
        .window(crate_name!(), WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .resizable()
        .build()
        .string_err()?;
    let mut canvas = window.into_canvas().present_vsync().build().string_err()?;
    canvas
        .set_logical_size(CANVAS_WIDTH, CANVAS_HEIGHT)
        .string_err()?;

    let texture_creator = canvas.texture_creator();

    let sky_texture = create_sky(
        &texture_creator,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        args.sky_color.into(),
        args.star_color.into(),
        args.stars,
        args.star_distance,
        args.moon_radius,
    )?;

    let mut buildings_canvas = create_surface_canvas(CANVAS_WIDTH, CANVAS_HEIGHT)?;

    let mut buildings_texture = texture_creator
        .create_texture_streaming(
            buildings_canvas.surface().pixel_format_enum(),
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
        )
        .string_err()?;
    buildings_texture.set_blend_mode(BlendMode::Blend);

    let mut generator = skyline(
        HEIGHT_RANGE,
        WIDTH_RANGE,
        args.windows,
        args.window_distance,
    );

    let mut event_pump = sdl_context.event_pump()?;

    let frame_length = Duration::from_secs(1) / args.fps;

    let mut last_frame = Instant::now();

    'main_loop: loop {
        let timeout = frame_length
            .checked_sub(last_frame.elapsed())
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u32;
        if let Some(event) = event_pump.wait_event_timeout(timeout) {
            match event {
                Event::Quit { .. } => break 'main_loop,
                _ => {}
            }
        }

        if last_frame.elapsed() >= frame_length {
            scroll_left(
                &mut buildings_canvas,
                &mut generator,
                args.border_color.into(),
                args.background_color.into(),
                args.window_color.into(),
            )?;

            buildings_canvas.surface().with_lock(|pixels| {
                buildings_texture
                    .update(
                        None,
                        pixels,
                        buildings_canvas.surface().pitch().try_into().unwrap(),
                    )
                    .string_err()
            })?;
            canvas.copy(&sky_texture, None, None)?;
            canvas.copy(&buildings_texture, None, None)?;
            canvas.present();

            last_frame = Instant::now();
        }
    }

    Ok(())
}

fn create_sky<T>(
    texture_creator: &TextureCreator<T>,
    width: u32,
    height: u32,
    sky_color: Color,
    star_color: Color,
    stars: usize,
    star_distance: u32,
    moon_radius: u32,
) -> Result<Texture, String> {
    let mut surface = Surface::new(width, height, PixelFormatEnum::RGBA32)?;

    surface.fill_rect(Rect::new(0, 0, width, height), sky_color)?;

    let mut canvas = surface.into_canvas()?;
    canvas.set_draw_color(star_color);

    for &(x, y) in sample_poisson_disc_2d(&mut thread_rng(), star_distance, width, height)
        .choose_multiple(&mut thread_rng(), stars)
    {
        canvas.draw_point(Point::new(x.try_into().unwrap(), y.try_into().unwrap()))?;
    }

    for (x, y) in filled_circle(MOON_CENTER, moon_radius) {
        canvas.draw_point(Point::new(x.try_into().unwrap(), y.try_into().unwrap()))?;
    }

    let surface = canvas.into_surface();

    let mut texture = texture_creator
        .create_texture_from_surface(surface)
        .string_err()?;

    texture.set_blend_mode(BlendMode::None);

    Ok(texture)
}

fn create_surface_canvas(width: u32, height: u32) -> Result<Canvas<Surface<'static>>, String> {
    let mut surface = Surface::new(width, height, PixelFormatEnum::RGBA32)?;
    surface.set_blend_mode(BlendMode::None)?;

    let mut canvas = surface.into_canvas()?;
    canvas.set_blend_mode(BlendMode::None);

    canvas.set_draw_color(TRANSPARENT);
    canvas.clear();

    Ok(canvas)
}

fn scroll_left(
    canvas: &mut Canvas<Surface>,
    generator: &mut impl Iterator<Item = Vec<Pixel>>,
    border_color: Color,
    background_color: Color,
    window_color: Color,
) -> Result<(), String> {
    let (width, height) = canvas.output_size()?;

    // Move all columns left by one position
    let source_rect = Rect::new(1, 0, width - 1, height);
    unsafe {
        let result = SDL_UpperBlit(
            canvas.surface_mut().raw(),
            source_rect.raw(),
            canvas.surface_mut().raw(),
            std::ptr::null_mut(),
        );
        if result != 0 {
            panic!("{}", sdl2::get_error());
        }
    }

    // Clear the rightmost column
    canvas.surface_mut().fill_rect(
        Rect::new((width - 1).try_into().unwrap(), 0, 1, height),
        TRANSPARENT,
    )?;

    // Pull a new column from the generator
    let new_column = generator.next().unwrap();
    let new_column_height: u32 = new_column.len().try_into().unwrap();
    for (pixel, row) in new_column
        .into_iter()
        .zip(height - new_column_height..height)
    {
        let color = match pixel {
            Pixel::Background => background_color,
            Pixel::Border => border_color,
            Pixel::Window => window_color,
        };

        let point = Point::new((width - 1).try_into().unwrap(), row.try_into().unwrap());

        canvas.set_draw_color(color);
        canvas.draw_point(point)?;
    }

    Ok(())
}

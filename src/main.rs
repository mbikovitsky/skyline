mod skyline;
mod util;

use std::{
    ops::Range,
    time::{Duration, Instant},
};

use clap::crate_name;
use rand::prelude::*;
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
const MAX_WINDOWS: usize = 5;
const WINDOW_MIN_DISTANCE: u32 = 2;
const CANVAS_WIDTH: u32 = 128;
const CANVAS_HEIGHT: u32 = 96;
const NUM_STARS: usize = 20;
const STAR_MIN_DISTANCE: u32 = 5;
const MOON_CENTER: (i32, i32) = (20, 20);
const MOON_RADIUS: u32 = 12;

const TRANSPARENT: Color = Color::RGBA(0, 0, 0, 0);
const SKY_COLOR: Color = Color::RGB(63, 63, 116);
const BORDER_COLOR: Color = Color::BLACK;
const BACKGROUND_COLOR: Color = Color::RGB(50, 60, 57);
const STAR_COLOR: Color = Color::WHITE;
const WINDOW_COLOR: Color = Color::RGB(251, 242, 54);

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const FPS: u32 = 30;

fn main() -> Result<(), String> {
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

    let sky_texture = create_sky(&texture_creator, CANVAS_WIDTH, CANVAS_HEIGHT)?;

    let mut buildings_canvas = create_surface_canvas(CANVAS_WIDTH, CANVAS_HEIGHT)?;

    let mut buildings_texture = texture_creator
        .create_texture_streaming(
            buildings_canvas.surface().pixel_format_enum(),
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
        )
        .string_err()?;
    buildings_texture.set_blend_mode(BlendMode::Blend);

    let mut generator = skyline(HEIGHT_RANGE, WIDTH_RANGE, MAX_WINDOWS, WINDOW_MIN_DISTANCE);

    let mut event_pump = sdl_context.event_pump()?;

    let frame_length = Duration::from_secs(1) / FPS;

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
            scroll_left(&mut buildings_canvas, &mut generator)?;

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
) -> Result<Texture, String> {
    let mut surface = Surface::new(width, height, PixelFormatEnum::RGBA32)?;

    surface.fill_rect(Rect::new(0, 0, width, height), SKY_COLOR)?;

    let mut canvas = surface.into_canvas()?;
    canvas.set_draw_color(STAR_COLOR);

    for &(x, y) in sample_poisson_disc_2d(&mut thread_rng(), STAR_MIN_DISTANCE, width, height)
        .choose_multiple(&mut thread_rng(), NUM_STARS)
    {
        canvas.draw_point(Point::new(x.try_into().unwrap(), y.try_into().unwrap()))?;
    }

    for (x, y) in filled_circle(MOON_CENTER, MOON_RADIUS) {
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
            Pixel::Background => BACKGROUND_COLOR,
            Pixel::Border => BORDER_COLOR,
            Pixel::Window => WINDOW_COLOR,
        };

        let point = Point::new((width - 1).try_into().unwrap(), row.try_into().unwrap());

        canvas.set_draw_color(color);
        canvas.draw_point(point)?;
    }

    Ok(())
}

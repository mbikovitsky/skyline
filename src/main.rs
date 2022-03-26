mod skyline;
mod util;

use std::{
    ops::RangeInclusive,
    time::{Duration, Instant},
};

use clap::crate_name;
use sdl2::{
    event::Event,
    pixels::Color,
    rect::{Point, Rect},
    render::Canvas,
    surface::Surface,
    sys::SDL_UpperBlit,
};

use skyline::{skyline, Pixel};
use util::StringErr;

const HEIGHT_RANGE: RangeInclusive<u32> = 5..=50;
const WIDTH_RANGE: RangeInclusive<u32> = 5..=10;

const SKY_COLOR: Color = Color::RGB(63, 63, 116);
const BORDER_COLOR: Color = Color::BLACK;
const BACKGROUND_COLOR: Color = Color::RGB(50, 60, 57);
// const WINDOW_COLOR: Color = Color::RGB(251, 242, 54);

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const CANVAS_WIDTH: u32 = 128;
const CANVAS_HEIGHT: u32 = 96;
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
    let mut output_texture = texture_creator
        .create_texture_streaming(None, CANVAS_WIDTH, CANVAS_HEIGHT)
        .string_err()?;

    let mut framebuffer =
        Surface::new(CANVAS_WIDTH, CANVAS_HEIGHT, canvas.default_pixel_format())?.into_canvas()?;
    framebuffer.set_draw_color(SKY_COLOR);
    framebuffer.clear();

    let mut generator = skyline(HEIGHT_RANGE, WIDTH_RANGE);

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
            scroll_left(&mut framebuffer, &mut generator)?;

            framebuffer.surface().with_lock(|pixels| {
                output_texture
                    .update(
                        None,
                        pixels,
                        framebuffer.surface().pitch().try_into().unwrap(),
                    )
                    .string_err()
            })?;
            canvas.copy(&output_texture, None, None)?;
            canvas.present();

            last_frame = Instant::now();
        }
    }

    Ok(())
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
        SKY_COLOR,
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
        };

        let point = Point::new((width - 1).try_into().unwrap(), row.try_into().unwrap());

        canvas.set_draw_color(color);
        canvas.draw_point(point)?;
    }

    Ok(())
}

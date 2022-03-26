mod skyline;

use std::time::{Duration, Instant};

use clap::crate_name;

use sdl2::{
    event::Event,
    pixels::Color,
    rect::{Point, Rect},
    render::Canvas,
    surface::Surface,
    sys::SDL_UpperBlit,
};
use skyline::{Pixel, RandomBuildingGenerator};

const MAX_BUILDING_HEIGHT: u32 = 50;
const MAX_BUILDING_WIDTH: u32 = 10;

const SKY_COLOR: Color = Color::RGB(63, 63, 116);
const BORDER_COLOR: Color = Color::BLACK;
const BACKGROUND_COLOR: Color = Color::RGB(50, 60, 57);
// const WINDOW_COLOR: Color = Color::RGB(251, 242, 54);

const WIDTH: u32 = 128;
const HEIGHT: u32 = 96;
const FPS: u32 = 12;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // 0 means nearest-neighbour
    // https://wiki.libsdl.org/SDL_HINT_RENDER_SCALE_QUALITY
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");

    let window = video_subsystem
        .window(crate_name!(), 800, 600)
        .position_centered()
        .resizable()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_logical_size(WIDTH, HEIGHT).unwrap();

    let texture_creator = canvas.texture_creator();
    let mut output_texture = texture_creator
        .create_texture_streaming(None, WIDTH, HEIGHT)
        .unwrap();

    let mut framebuffer = Surface::new(WIDTH, HEIGHT, canvas.default_pixel_format())
        .unwrap()
        .into_canvas()
        .unwrap();
    framebuffer.set_draw_color(SKY_COLOR);
    framebuffer.clear();

    let mut generator =
        RandomBuildingGenerator::new(5..=MAX_BUILDING_HEIGHT, 5..=MAX_BUILDING_WIDTH)
            .map(|building| building.iter_columns())
            .flatten();

    let mut event_pump = sdl_context.event_pump().unwrap();

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
            scroll_left(&mut framebuffer, &mut generator);

            framebuffer.surface().with_lock(|pixels| {
                output_texture
                    .update(
                        None,
                        pixels,
                        framebuffer.surface().pitch().try_into().unwrap(),
                    )
                    .unwrap()
            });
            canvas.copy(&output_texture, None, None).unwrap();
            canvas.present();

            last_frame = Instant::now();
        }
    }
}

fn scroll_left(canvas: &mut Canvas<Surface>, generator: &mut impl Iterator<Item = Vec<Pixel>>) {
    let (width, height) = canvas.output_size().unwrap();

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
    canvas
        .surface_mut()
        .fill_rect(
            Rect::new((width - 1).try_into().unwrap(), 0, 1, height),
            SKY_COLOR,
        )
        .unwrap();

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
        canvas.draw_point(point).unwrap();
    }
}
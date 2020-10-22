pub(crate) use eyre::*;

mod render;
#[macro_use]
mod utils;

const FPS: u32 = 60;
const FRAME_UPDATE_TIME: f64 = 1.0 / FPS as f64;

fn main() -> Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let mut window = minifb::Window::new(
        "Fancy Pong",
        800,
        800,
        minifb::WindowOptions {
            resize: true,
            scale: minifb::Scale::X2,
            ..Default::default()
        },
    )?;
    let window_handle = window.get_window_handle();
    let size = window.get_size();

    let a = (0, 0);

    let renderer =
        futures::executor::block_on(render::Renderer::new(&window, tuple_as!(size, (u32, u32))))?;

    window.limit_update_rate(Some(std::time::Duration::from_secs_f64(FRAME_UPDATE_TIME)));
    let size = window.get_size();

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        window.update();
    }

    Ok(())
}

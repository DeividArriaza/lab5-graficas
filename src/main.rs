mod color;
mod framebuffer;
mod mesh;
mod png;
mod renderer;
mod shaders;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use png::WritePng;
use renderer::{Renderer, HEIGHT, WIDTH};
use shaders::{BodyKind, ALL_BODIES};
use std::{
    error::Error,
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Options {
    body: BodyKind,
    time: f32,
    output: Option<String>,
    render_all: bool,
    headless: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_args(&std::env::args().skip(1).collect::<Vec<_>>())?;
    let mut renderer = Renderer::new();
    if options.render_all {
        let directory = options.output.as_deref().unwrap_or("screenshots");
        for body in ALL_BODIES {
            save(
                &mut renderer,
                body,
                options.time,
                &Path::new(directory).join(format!("{}.png", body.slug())),
            )?;
        }
        return save_gallery(
            &mut renderer,
            options.time,
            &Path::new(directory).join("gallery.png"),
        );
    }
    if let Some(path) = options.output {
        return save(&mut renderer, options.body, options.time, Path::new(&path));
    }
    if options.headless {
        return Err("--headless requires --output or --render-all".into());
    }
    run_window(options.body, options.time, &mut renderer)
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        body: BodyKind::Sun,
        time: 0.0,
        output: None,
        render_all: false,
        headless: false,
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--render-all" => options.render_all = true,
            "--headless" => options.headless = true,
            "--body" | "--output" | "--time" => {
                let value = args
                    .next()
                    .filter(|s| !s.is_empty() && !s.starts_with("--"))
                    .ok_or_else(|| format!("{flag} requires a value"))?;
                match flag.as_str() {
                    "--body" => {
                        options.body = ALL_BODIES
                            .iter()
                            .copied()
                            .find(|body| body.slug() == value)
                            .ok_or_else(|| format!("unknown body: {value}"))?
                    }
                    "--output" => options.output = Some(value.clone()),
                    _ => {
                        options.time = value
                            .parse::<f32>()
                            .map_err(|_| format!("invalid time: {value}"))?;
                        if !options.time.is_finite() {
                            return Err(format!("time must be finite: {value}"));
                        }
                    }
                }
            }
            _ => return Err(format!("unknown flag: {flag}")),
        }
    }
    Ok(options)
}

fn run_window(body: BodyKind, time: f32, renderer: &mut Renderer) -> Result<(), Box<dyn Error>> {
    let mut window = Window::new(body.name(), WIDTH, HEIGHT, WindowOptions::default())?;
    let mut selected = body;
    let mut paused = false;
    let mut translating = false;
    let mut last = Instant::now();
    let mut elapsed = time;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();
        let dt = frame_start.duration_since(last).as_secs_f32().min(0.05);
        last = frame_start;
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            paused = !paused;
        }
        if !paused {
            elapsed += dt;
        }
        if window.is_key_pressed(Key::O, KeyRepeat::No) {
            translating = !translating;
        }
        if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
            let index = ALL_BODIES.iter().position(|b| *b == selected).unwrap_or(0);
            selected = ALL_BODIES[(index + 1) % ALL_BODIES.len()];
        }
        for (key, body) in [
            Key::Key1,
            Key::Key2,
            Key::Key3,
            Key::Key4,
            Key::Key5,
            Key::Key6,
        ]
        .into_iter()
        .zip(ALL_BODIES)
        {
            if window.is_key_pressed(key, KeyRepeat::No) {
                selected = body;
            }
        }
        let horizontal =
            i32::from(window.is_key_down(Key::Right)) - i32::from(window.is_key_down(Key::Left));
        let vertical =
            i32::from(window.is_key_down(Key::Up)) - i32::from(window.is_key_down(Key::Down));
        renderer.rotation.y += horizontal as f32 * dt * 1.5;
        renderer.rotation.x += vertical as f32 * dt * 1.5;
        renderer.translation = if translating {
            nalgebra_glm::vec3(
                (elapsed * 0.45).sin() * 100.0,
                (elapsed * 0.7).sin() * 35.0,
                0.0,
            )
        } else {
            nalgebra_glm::vec3(0.0, 0.0, 0.0)
        };
        renderer.render(selected, elapsed);
        if window.is_key_pressed(Key::S, KeyRepeat::No) {
            save_frame(renderer, Path::new("screenshots/capture.png"))?;
        }
        window.set_title(&format!(
            "{} | 1-6/Tab: cuerpo | Space: pausa | flechas: rotar | O: trasladar | S: PNG",
            selected.name()
        ));
        window.update_with_buffer(&renderer.framebuffer.buffer, WIDTH, HEIGHT)?;
        if let Some(remaining) =
            Duration::from_secs_f64(1.0 / 60.0).checked_sub(frame_start.elapsed())
        {
            std::thread::sleep(remaining);
        }
    }
    Ok(())
}

fn create_parent(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn save_frame(renderer: &Renderer, path: &Path) -> Result<(), Box<dyn Error>> {
    create_parent(path)?;
    renderer
        .framebuffer
        .render_png(path.to_str().ok_or("output path is not UTF-8")?)?;
    println!("Saved {}", path.display());
    Ok(())
}

fn save(
    renderer: &mut Renderer,
    body: BodyKind,
    time: f32,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    renderer.render(body, time);
    save_frame(renderer, path)
}

fn copy_gallery_tile(gallery: &mut [u32], tile: &[u32], index: usize) {
    let left = index % 3 * WIDTH;
    let top = index / 3 * HEIGHT;
    for y in 0..HEIGHT {
        let start = (top + y) * WIDTH * 3 + left;
        gallery[start..start + WIDTH].copy_from_slice(&tile[y * WIDTH..(y + 1) * WIDTH]);
    }
}

fn save_gallery(renderer: &mut Renderer, time: f32, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut gallery = vec![0; WIDTH * 3 * HEIGHT * 2];
    for (index, body) in ALL_BODIES.into_iter().enumerate() {
        renderer.render(body, time);
        copy_gallery_tile(&mut gallery, &renderer.framebuffer.buffer, index);
    }
    create_parent(path)?;
    png::write_png_file(
        path.to_str().ok_or("output path is not UTF-8")?,
        &gallery,
        WIDTH * 3,
        HEIGHT * 2,
    )?;
    println!("Saved {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn invalid_and_nonfinite_times_are_rejected() {
        for value in ["NaN", "inf", "-inf", "1e99", "nope", "", "0.1.2"] {
            assert!(parse(&["--time", value]).is_err(), "{value}");
        }
    }
    #[test]
    fn unknown_and_missing_arguments_are_rejected() {
        for args in [
            vec!["--wat"],
            vec!["--time"],
            vec!["--body"],
            vec!["--output"],
            vec!["--body", "pluto"],
            vec!["--output", "--headless"],
            vec!["--output", ""],
        ] {
            assert!(parse(&args).is_err());
        }
    }
    #[test]
    fn parses_six_bodies_and_finite_negative_time() {
        for body in ALL_BODIES {
            let options = parse(&[
                "--body",
                body.slug(),
                "--time",
                "-2.5",
                "--headless",
                "--output",
                "a/b.png",
            ])
            .unwrap();
            assert_eq!(options.body, body);
            assert_eq!(options.time, -2.5);
            assert!(options.headless);
            assert_eq!(options.output.as_deref(), Some("a/b.png"));
        }
        assert!(parse(&["--render-all"]).unwrap().render_all);
    }
    #[test]
    fn gallery_places_all_six_full_tiles_without_overlap() {
        let mut gallery = vec![0; WIDTH * 3 * HEIGHT * 2];
        for index in 0..6 {
            copy_gallery_tile(
                &mut gallery,
                &vec![(index + 1) as u32; WIDTH * HEIGHT],
                index,
            );
        }
        for y in 0..HEIGHT * 2 {
            for x in 0..WIDTH * 3 {
                assert_eq!(
                    gallery[y * WIDTH * 3 + x],
                    (y / HEIGHT * 3 + x / WIDTH + 1) as u32
                );
            }
        }
    }
}

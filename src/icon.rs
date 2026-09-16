const WINDOW: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/window.rgba"));
const TRAY_LOCKED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-locked.rgba"));
const TRAY_UNLOCKED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-unlocked.rgba"));

pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub fn window() -> Rgba {
    square(WINDOW)
}

pub fn tray(locked: bool) -> Rgba {
    square(if locked { TRAY_LOCKED } else { TRAY_UNLOCKED })
}

fn square(pixels: &[u8]) -> Rgba {
    let side = ((pixels.len() / 4) as f64).sqrt() as u32;
    Rgba {
        width: side,
        height: side,
        pixels: pixels.to_vec(),
    }
}

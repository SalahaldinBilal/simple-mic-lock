use std::path::Path;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

const TRAY_ICON_SIZE: u32 = 32;
const UNLOCKED_RECOLOR: [(&str, &str); 2] = [("#5BDD99", "#959CA7"), ("#2EB06B", "#6A717C")];

fn main() {
    println!("cargo:rerun-if-changed=../assets/tray.svg");

    let out = std::env::var("OUT_DIR").unwrap();
    let tray = std::fs::read_to_string("../assets/tray.svg").expect("cannot read assets/tray.svg");

    write_rgba(&render(&tray), &Path::new(&out).join("tray-locked.rgba"));
    write_rgba(&render(&unlocked(&tray)), &Path::new(&out).join("tray-unlocked.rgba"));

    tauri_build::build()
}

fn unlocked(svg: &str) -> String {
    let mut recolored = svg.to_string();
    for (locked, grey) in UNLOCKED_RECOLOR {
        assert!(recolored.contains(locked), "assets/tray.svg no longer uses {locked}");
        recolored = recolored.replace(locked, grey);
    }
    recolored
}

fn render(svg: &str) -> Pixmap {
    let tree = Tree::from_str(svg, &Options::default()).expect("tray SVG failed to parse");
    let mut pixmap = Pixmap::new(TRAY_ICON_SIZE, TRAY_ICON_SIZE).unwrap();
    let scale = TRAY_ICON_SIZE as f32 / tree.size().width();
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap
}

// tiny-skia stores premultiplied alpha, tray icons expect straight alpha.
fn write_rgba(pixmap: &Pixmap, destination: &Path) {
    let mut rgba = pixmap.data().to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = pixel[3] as u32;
        if alpha > 0 && alpha < 255 {
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    std::fs::write(destination, rgba).unwrap();
}

use std::path::{Path, PathBuf};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

const APP_NAME: &str = "Simple Mic Lock";
const WINDOW_ICON_SIZE: u32 = 64;
const TRAY_ICON_SIZE: u32 = 32;
const EXE_ICON_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];
const UNLOCKED_RECOLOR: [(&str, &str); 2] = [("#5BDD99", "#959CA7"), ("#2EB06B", "#6A717C")];

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rustc-env=APP_NAME={APP_NAME}");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let icon = read("assets/icon.svg");
    let tray = read("assets/tray.svg");

    write_rgba(&render(&icon, WINDOW_ICON_SIZE), &out.join("window.rgba"));
    write_rgba(&render(&tray, TRAY_ICON_SIZE), &out.join("tray-locked.rgba"));
    write_rgba(&render(&unlocked(&tray), TRAY_ICON_SIZE), &out.join("tray-unlocked.rgba"));

    embed_resources(&out, &icon);
    embed_manifest();
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

fn package(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|error| panic!("cargo did not set {key}: {error}"))
}

fn embed_manifest() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }

    let manifest = Path::new(&package("CARGO_MANIFEST_DIR")).join("app.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
    println!("cargo:rustc-link-arg-bins=/MANIFESTUAC:level='asInvoker' uiAccess='false'");
}

fn embed_resources(out: &Path, icon: &str) {
    let images: Vec<(u32, Vec<u8>)> = EXE_ICON_SIZES
        .iter()
        .map(|&size| (size, render(icon, size).encode_png().unwrap()))
        .collect();

    let ico = out.join("app.ico");
    std::fs::write(&ico, ico_file(&images)).unwrap();
    let ico_path = ico.display().to_string().replace('\\', "\\\\");

    let major = package("CARGO_PKG_VERSION_MAJOR");
    let minor = package("CARGO_PKG_VERSION_MINOR");
    let patch = package("CARGO_PKG_VERSION_PATCH");
    let version = package("CARGO_PKG_VERSION");
    let exe = format!("{}.exe", package("CARGO_PKG_NAME"));

    // FILEOS is VOS_NT_WINDOWS32 and FILETYPE is VFT_APP.
    let script = format!(
        r#"1 ICON "{ico_path}"

1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "{APP_NAME}"
      VALUE "ProductName", "{APP_NAME}"
      VALUE "FileVersion", "{version}"
      VALUE "ProductVersion", "{version}"
      VALUE "InternalName", "{exe}"
      VALUE "OriginalFilename", "{exe}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );

    let rc = out.join("app.rc");
    std::fs::write(&rc, script).unwrap();
    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}

fn ico_file(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    const HEADER_LEN: usize = 6;
    const ENTRY_LEN: usize = 16;

    let mut file = Vec::new();
    file.extend_from_slice(&0u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = HEADER_LEN + ENTRY_LEN * images.len();
    for (size, png) in images {
        // ICO stores a 256px dimension as 0.
        let dimension = if *size >= 256 { 0 } else { *size as u8 };
        file.extend_from_slice(&[dimension, dimension, 0, 0]);
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&(png.len() as u32).to_le_bytes());
        file.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for (_, png) in images {
        file.extend_from_slice(png);
    }
    file
}

fn unlocked(svg: &str) -> String {
    let mut recolored = svg.to_string();
    for (locked, grey) in UNLOCKED_RECOLOR {
        assert!(recolored.contains(locked), "assets/tray.svg no longer uses {locked}");
        recolored = recolored.replace(locked, grey);
    }
    recolored
}

fn render(svg: &str, size: u32) -> Pixmap {
    let tree = Tree::from_str(svg, &Options::default()).expect("icon SVG failed to parse");
    let mut pixmap = Pixmap::new(size, size).unwrap();
    let scale = size as f32 / tree.size().width();
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap
}

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

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, TextStyle};

pub const SEMIBOLD: &str = "semibold";

pub struct Palette {
    pub background: Color32,
    pub card: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub track: Color32,
    pub knob: Color32,
    pub warning: Color32,
}

pub const PALETTE: Palette = Palette {
    background: Color32::from_rgb(0x14, 0x16, 0x1A),
    card: Color32::from_rgb(0x1C, 0x1F, 0x25),
    outline: Color32::from_rgb(0x2A, 0x2F, 0x37),
    text: Color32::from_rgb(0xE8, 0xEA, 0xEE),
    muted: Color32::from_rgb(0x8D, 0x95, 0xA1),
    accent: Color32::from_rgb(0x3B, 0xC9, 0x7E),
    accent_soft: Color32::from_rgb(0x1D, 0x3A, 0x2C),
    track: Color32::from_rgb(0x2C, 0x31, 0x3A),
    knob: Color32::from_rgb(0xF4, 0xF6, 0xF8),
    warning: Color32::from_rgb(0xE0, 0x6C, 0x5C),
};

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(egui::ThemePreference::Dark);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.scroll.floating = false;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 6.0;
        style.text_styles = [
            (TextStyle::Heading, semibold(16.0)),
            (TextStyle::Body, FontId::proportional(13.5)),
            (TextStyle::Button, semibold(13.0)),
            (TextStyle::Small, FontId::proportional(11.5)),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into();
    });

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PALETTE.background;
    visuals.window_fill = PALETTE.card;
    visuals.extreme_bg_color = PALETTE.track;
    visuals.override_text_color = Some(PALETTE.text);
    visuals.selection.bg_fill = PALETTE.accent;

    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.corner_radius = CornerRadius::same(8);
        widget.bg_stroke = egui::Stroke::new(1.0, PALETTE.outline);
    }
    // bg_fill also colours the scroll bar handle.
    visuals.widgets.inactive.bg_fill = PALETTE.track;
    visuals.widgets.hovered.bg_fill = PALETTE.muted;
    visuals.widgets.active.bg_fill = PALETTE.muted;
    visuals.widgets.inactive.weak_bg_fill = PALETTE.card;
    visuals.widgets.hovered.weak_bg_fill = PALETTE.accent_soft;
    visuals.widgets.active.weak_bg_fill = PALETTE.accent_soft;
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);

    ctx.set_visuals_of(egui::Theme::Dark, visuals);
}

fn install_fonts(ctx: &egui::Context) {
    const REGULAR: &str = r"C:\Windows\Fonts\segoeui.ttf";
    const SEMIBOLD_FILES: [&str; 2] =
        [r"C:\Windows\Fonts\seguisb.ttf", r"C:\Windows\Fonts\segoeuib.ttf"];

    let mut fonts = FontDefinitions::default();

    if let Ok(bytes) = std::fs::read(REGULAR) {
        fonts.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".into());
    }

    let heavier = SEMIBOLD_FILES.iter().find_map(|path| std::fs::read(path).ok());
    if let Some(bytes) = heavier {
        fonts.font_data.insert("segoe-semibold".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .insert(FontFamily::Name(SEMIBOLD.into()), vec!["segoe-semibold".into()]);
    } else {
        let fallback = fonts.families[&FontFamily::Proportional].clone();
        fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), fallback);
    }

    ctx.set_fonts(fonts);
}

use std::sync::Arc;

use egui::{
    Align, Color32, CornerRadius, Layout, Margin, Pos2, Rect, RichText, Stroke, pos2, vec2,
};

use crate::state::{Device, Prompt, State, Unidentified};
use crate::theme::{PALETTE, semibold};
use crate::{APP_NAME, icon};

const WINDOW_WIDTH: f32 = 440.0;
const MAX_WINDOW_HEIGHT: f32 = 820.0;
const DEVICE_LIST_MAX_HEIGHT: f32 = 430.0;
const KNOB_RADIUS: f32 = 9.0;
const ICON_BUTTON: f32 = 32.0;
const TARGET_LABEL_WIDTH: f32 = 46.0;

pub fn run(state: Arc<State>, hidden: bool) {
    let art = icon::window();
    let window_icon = egui::IconData {
        rgba: art.pixels,
        width: art.width,
        height: art.height,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size([WINDOW_WIDTH, 420.0])
            .with_min_inner_size([WINDOW_WIDTH, 240.0])
            .with_maximize_button(false)
            .with_visible(!hidden)
            .with_icon(window_icon),
        ..Default::default()
    };

    let _ = eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            crate::theme::install(&cc.egui_ctx);
            state.attach_ui(cc.egui_ctx.clone());
            Ok(Box::new(Window::new(state)))
        }),
    );
}

struct Rename {
    key: String,
    draft: String,
    focused: bool,
}

struct Window {
    state: Arc<State>,
    autorun_error: Option<String>,
    applied_height: f32,
    renaming: Option<Rename>,
}

impl Window {
    fn new(state: Arc<State>) -> Self {
        Self { state, autorun_error: None, applied_height: 0.0, renaming: None }
    }
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if ctx.input(|i| i.viewport().close_requested()) && !self.state.quitting() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.state.hide_window();
        }

        let devices = self.state.devices();
        let unidentified = self.state.unidentified();
        let mut content_height = 0.0;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(PALETTE.background).inner_margin(Margin::same(20)))
            .show(ui, |ui| {
                let top = ui.min_rect().top();

                header(ui, &self.state);
                ui.add_space(16.0);
                self.startup_card(ui);
                ui.add_space(18.0);
                caption(ui, "MICROPHONES");
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .max_height(DEVICE_LIST_MAX_HEIGHT)
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .show(ui, |ui| {
                        if devices.is_empty() && unidentified.is_empty() {
                            card(ui, |ui| {
                                ui.label(
                                    RichText::new("No recording device found")
                                        .color(PALETTE.muted),
                                );
                            });
                        }

                        let mut first = true;
                        for unit in &unidentified {
                            spacing(ui, &mut first);
                            self.unidentified_card(ui, unit);
                        }
                        for device in &devices {
                            spacing(ui, &mut first);
                            self.device_card(ui, device);
                        }
                    });

                ui.add_space(18.0);
                self.footer(ui);
                content_height = ui.min_rect().bottom() - top + 40.0;
            });

        self.fit_window(&ctx, content_height);
    }
}

impl Window {
    fn fit_window(&mut self, ctx: &egui::Context, content_height: f32) {
        let wanted = content_height.clamp(240.0, MAX_WINDOW_HEIGHT).round();
        if (wanted - self.applied_height).abs() < 2.0 {
            return;
        }
        self.applied_height = wanted;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(WINDOW_WIDTH, wanted)));
    }

    fn device_card(&mut self, ui: &mut egui::Ui, device: &Device) {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - ICON_BUTTON * 2.0 - 16.0);
                    self.device_name(ui, device);
                    ui.add_space(1.0);
                    ui.label(RichText::new(subtitle(device)).small().color(PALETTE.muted));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if lock_button(ui, device.locked, device.adjustable) {
                        self.state.set_locked(&device.key, !device.locked);
                    }
                    if rename_button(ui) {
                        self.renaming = Some(Rename {
                            key: device.key.clone(),
                            draft: device.display_name().to_owned(),
                            focused: false,
                        });
                    }
                });
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let width = ui.available_width() - TARGET_LABEL_WIDTH - 8.0;
                let mut target = device.target;
                if slider(ui, &mut target, device, width) {
                    self.state.set_target(&device.key, target);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let color = if device.locked { PALETTE.text } else { PALETTE.muted };
                    ui.label(RichText::new(format!("{target}%")).font(semibold(14.0)).color(color));
                });
            });
        });
    }

    fn device_name(&mut self, ui: &mut egui::Ui, device: &Device) {
        let Some(rename) = self.renaming.as_mut().filter(|rename| rename.key == device.key) else {
            let color = if device.present() { PALETTE.text } else { PALETTE.muted };
            ui.label(RichText::new(device.display_name()).font(semibold(13.5)).color(color));
            return;
        };

        let width = ui.available_width();
        let response = ui.add(
            egui::TextEdit::singleline(&mut rename.draft)
                .font(semibold(13.5))
                .hint_text(RichText::new(&device.name).color(PALETTE.muted))
                .desired_width(width)
                .margin(Margin::symmetric(6, 3)),
        );
        if !rename.focused {
            response.request_focus();
            rename.focused = true;
        }

        if response.lost_focus() {
            let cancelled = ui.input(|input| input.key_pressed(egui::Key::Escape));
            let draft = rename.draft.trim().to_owned();
            self.renaming = None;
            if !cancelled {
                let nickname = (!draft.is_empty() && draft != device.name).then_some(draft);
                self.state.set_nickname(&device.key, nickname);
            }
        }
    }

    fn unidentified_card(&mut self, ui: &mut egui::Ui, unit: &Unidentified) {
        let found = &unit.discovered;
        card(ui, |ui| {
            let name = found.name.as_deref().unwrap_or(&found.endpoint);
            ui.label(RichText::new(name).font(semibold(13.5)).color(PALETTE.text));
            ui.add_space(1.0);
            ui.label(RichText::new("Not identified yet").small().color(PALETTE.warning));
            ui.add_space(10.0);

            match &unit.prompt {
                Prompt::Choose(candidates) => {
                    ui.label(RichText::new("Which mic is this?").color(PALETTE.text));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for candidate in candidates {
                            if choice_button(ui, &candidate.name) {
                                self.state.claim(&found.endpoint, Some(candidate.key.clone()));
                            }
                        }
                        if choice_button(ui, "New mic") {
                            self.state.claim(&found.endpoint, None);
                        }
                    });
                }
                Prompt::KeepOnlyOne(Some(saved)) => {
                    ui.label(
                        RichText::new(format!(
                            "Identical mics can't be told apart. Keep only {saved} plugged in and unplug the other one, then plug it back in."
                        ))
                        .color(PALETTE.text),
                    );
                }
                Prompt::KeepOnlyOne(None) => {
                    ui.label(
                        RichText::new(
                            "Identical mics can't be told apart. Keep only one plugged in, then reconnect the others one at a time.",
                        )
                        .color(PALETTE.text),
                    );
                }
            }
        });
    }

    fn startup_card(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            let autorun = self.state.autorun();
            if switch_row(
                ui,
                "Start with Windows",
                "Launches straight to the tray when you sign in",
                autorun,
            ) {
                self.autorun_error = self.state.set_autorun(!autorun).err();
            }

            if let Some(error) = &self.autorun_error {
                ui.add_space(8.0);
                ui.label(RichText::new(error).small().color(PALETTE.warning));
            }
        });
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let quit = ui.add(
                egui::Button::new(RichText::new("Quit").color(PALETTE.muted))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            );
            pointer_over(ui, quit.rect, false);
            if quit.clicked() {
                self.state.request_quit();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let hide = ui.add(
                    egui::Button::new(RichText::new("Hide to tray").color(PALETTE.knob))
                        .fill(PALETTE.accent)
                        .stroke(Stroke::NONE),
                );
                pointer_over(ui, hide.rect, false);
                if hide.clicked() {
                    self.state.hide_window();
                }
            });
        });
    }
}

fn spacing(ui: &mut egui::Ui, first: &mut bool) {
    if !*first {
        ui.add_space(10.0);
    }
    *first = false;
}

fn header(ui: &mut egui::Ui, state: &State) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(APP_NAME).font(semibold(17.0)).color(PALETTE.text));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (label, color) = match state.locked_count() {
                (_, 0) => ("NO MIC".to_string(), PALETTE.muted),
                (0, _) => ("NOTHING LOCKED".to_string(), PALETTE.muted),
                (locked, total) if locked == total => ("LOCKED".to_string(), PALETTE.accent),
                (locked, total) => (format!("{locked} OF {total} LOCKED"), PALETTE.accent),
            };
            pill(ui, &label, color);
        });
    });
}

fn subtitle(device: &Device) -> String {
    let mut parts = Vec::new();
    if device.nickname.is_some() {
        parts.push(device.name.clone());
    }
    if device.is_default {
        parts.push("Default".to_string());
    }
    if !device.present() {
        parts.push("Not connected".into());
    } else if !device.adjustable {
        parts.push("No volume control".into());
    } else if let Some(level) = device.level {
        parts.push(format!("now {}%", (level * 100.0).round() as i32));
    }
    parts.join(" · ")
}

fn card(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(PALETTE.card)
        .stroke(Stroke::new(1.0, PALETTE.outline))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contents(ui);
        });
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(semibold(10.5)).color(PALETTE.muted));
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(if color == PALETTE.accent { PALETTE.accent_soft } else { PALETTE.track })
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(semibold(10.0)).color(color));
        });
}

fn choice_button(ui: &mut egui::Ui, text: &str) -> bool {
    let response = ui.add(
        egui::Button::new(RichText::new(text).color(PALETTE.text))
            .fill(PALETTE.track)
            .stroke(Stroke::NONE),
    );
    pointer_over(ui, response.rect, false);
    response.clicked()
}

fn rename_button(ui: &mut egui::Ui) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ICON_BUTTON, ICON_BUTTON), egui::Sense::click());

    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(9), PALETTE.track);
    }
    let centre = rect.center();
    let tip = centre + vec2(-5.0, 5.0);
    let shoulder = centre + vec2(-2.2, 2.2);
    let end = centre + vec2(5.0, -5.0);
    painter.line_segment([shoulder, end], Stroke::new(3.4, PALETTE.muted));
    painter.line_segment([tip, shoulder], Stroke::new(1.4, PALETTE.muted));

    pointer_over(ui, rect, false);
    response.clicked()
}

fn lock_button(ui: &mut egui::Ui, locked: bool, enabled: bool) -> bool {
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(ICON_BUTTON, ICON_BUTTON), sense);

    let background = match (locked, enabled, response.hovered()) {
        (true, true, _) => PALETTE.accent_soft,
        (true, false, _) => PALETTE.track,
        (false, true, true) => PALETTE.track,
        _ => Color32::TRANSPARENT,
    };

    let color = if enabled && locked { PALETTE.accent } else { PALETTE.muted };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(9), background);
    padlock(painter, rect.center(), locked, color);

    if enabled {
        pointer_over(ui, rect, false);
    }
    response.clicked()
}

fn padlock(painter: &egui::Painter, centre: Pos2, locked: bool, color: Color32) {
    let body = Rect::from_center_size(pos2(centre.x, centre.y + 3.0), vec2(13.0, 10.0));
    painter.rect_filled(body, CornerRadius::same(2), color);
    painter.circle_filled(body.center(), 1.4, PALETTE.card);

    let radius = 4.0;
    let pivot = if locked {
        pos2(centre.x, body.top())
    } else {
        pos2(centre.x + 2.5, body.top() - 1.0)
    };
    let sweep_end = if locked { 0.0 } else { std::f32::consts::PI * 0.30 };

    let shackle: Vec<Pos2> = (0..=16)
        .map(|step| {
            let t = step as f32 / 16.0;
            let angle = std::f32::consts::PI + (sweep_end - std::f32::consts::PI) * t;
            pos2(pivot.x + radius * angle.cos(), pivot.y - radius * angle.sin())
        })
        .collect();
    painter.add(egui::Shape::line(shackle, Stroke::new(2.0, color)));
}

fn slider(ui: &mut egui::Ui, value: &mut u32, device: &Device, width: f32) -> bool {
    let enabled = device.adjustable;
    let sense = if enabled { egui::Sense::click_and_drag() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(width, KNOB_RADIUS * 2.0), sense);
    let rail = Rect::from_center_size(rect.center(), vec2(rect.width() - KNOB_RADIUS * 2.0, 6.0));

    let mut changed = false;
    if let Some(pointer) = response.interact_pointer_pos() {
        let fraction = ((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0);
        let picked = (fraction * 100.0).round() as u32;
        if picked != *value {
            *value = picked;
            changed = true;
        }
    }

    let fraction = *value as f32 / 100.0;
    let knob = pos2(rail.left() + rail.width() * fraction, rail.center().y);
    let fill = if enabled && device.locked { PALETTE.accent } else { PALETTE.muted };

    let painter = ui.painter();
    painter.rect_filled(rail, CornerRadius::same(3), PALETTE.track);
    painter.rect_filled(
        Rect::from_min_max(rail.min, pos2(knob.x, rail.bottom())),
        CornerRadius::same(3),
        fill,
    );

    if let Some(level) = device.level
        && (level - fraction).abs() > 0.01
    {
        let drifted_to = rail.left() + rail.width() * level;
        painter.rect_filled(
            Rect::from_center_size(
                pos2(drifted_to, rail.center().y),
                vec2(2.0, rail.height() + 8.0),
            ),
            CornerRadius::same(1),
            PALETTE.text,
        );
    }

    painter.circle_filled(knob, KNOB_RADIUS, PALETTE.knob);
    painter.circle_stroke(knob, KNOB_RADIUS, Stroke::new(1.0, PALETTE.outline));

    if enabled {
        pointer_over(ui, rect, response.dragged());
    }
    changed
}

fn switch_row(ui: &mut egui::Ui, title: &str, detail: &str, on: bool) -> bool {
    let mut toggled = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 56.0);
            ui.label(RichText::new(title).color(PALETTE.text));
            ui.add_space(1.0);
            ui.label(RichText::new(detail).small().color(PALETTE.muted));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggled = switch(ui, on);
        });
    });
    toggled
}

fn switch(ui: &mut egui::Ui, on: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(42.0, 24.0), egui::Sense::click());
    let travel = ui.ctx().animate_bool_with_time(response.id, on, 0.12);

    let radius = rect.height() / 2.0;
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(radius as u8),
        if on { PALETTE.accent } else { PALETTE.track },
    );
    let centre = pos2(
        egui::lerp((rect.left() + radius)..=(rect.right() - radius), travel),
        rect.center().y,
    );
    painter.circle_filled(centre, radius - 3.0, PALETTE.knob);

    pointer_over(ui, rect, false);
    response.clicked()
}

// egui pads hit areas by interact_radius, so the cursor tracks the drawn bounds instead.
fn pointer_over(ui: &egui::Ui, bounds: Rect, force: bool) {
    let inside = ui.ctx().pointer_hover_pos().is_some_and(|at| bounds.contains(at));
    if force || inside {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

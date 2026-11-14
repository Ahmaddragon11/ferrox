// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::theme;
use ferrox_core::{ComputeMode, TabId, TabState};
use egui::{Align, Color32, Key, Layout, RichText, Sense, TextEdit, Ui};

#[derive(Debug, Clone)]
pub struct ChromeTab {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub state: TabState,
    pub loading: bool,
    pub muted: bool,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct SuggestionItem {
    pub kind: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeAction {
    NewTab,
    ActivateTab(TabId),
    CloseTab(TabId),
    Navigate(String),
    AddressChanged,
    Back,
    Forward,
    Reload,
    Home,
    ToggleShield,
    CycleComputeMode,
    ToggleIncognito,
    OpenInternal(String),
    CheckUpdates,
    Quit,
}

#[derive(Debug)]
pub struct ChromeState {
    pub tabs: Vec<ChromeTab>,
    pub active: Option<TabId>,
    pub address: String,
    pub suggestions: Vec<SuggestionItem>,
    pub selected: usize,
    pub show_suggestions: bool,
    pub shield_enabled: bool,
    pub blocked: u64,
    pub compute_mode: ComputeMode,
    pub incognito: bool,
    pub update_available: bool,
    pub loading: bool,
    pub can_back: bool,
    pub can_forward: bool,
    pub menu_open: bool,
    pub focus_address: bool,
    pub status: Option<String>,
    pub memory_mb: f64,
    pub version: String,
}

impl Default for ChromeState {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            address: String::new(),
            suggestions: Vec::new(),
            selected: 0,
            show_suggestions: false,
            shield_enabled: true,
            blocked: 0,
            compute_mode: ComputeMode::Balanced,
            incognito: false,
            update_available: false,
            loading: false,
            can_back: false,
            can_forward: false,
            menu_open: false,
            focus_address: false,
            status: None,
            memory_mb: 48.0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

const TABS_H: f32 = 30.0;
const NAV_H: f32 = 36.0;
const SUGG_ROW: f32 = 25.0;
const MENU_H: f32 = 30.0;
const PAD: f32 = 7.0;

pub fn chrome_height(state: &ChromeState) -> f32 {
    let suggestions = if state.show_suggestions && !state.suggestions.is_empty() {
        state.suggestions.len().min(6) as f32 * SUGG_ROW + 4.0
    } else {
        0.0
    };
    let menu = if state.menu_open { MENU_H + 4.0 } else { 0.0 };
    PAD + TABS_H + 3.0 + NAV_H + suggestions + menu + PAD
}

fn compute_color(mode: ComputeMode) -> Color32 {
    match mode {
        ComputeMode::Turbo => theme::TEAL,
        ComputeMode::Eco => theme::GREEN,
        ComputeMode::Balanced => theme::ACCENT_SOFT,
        ComputeMode::Full => theme::MUTED,
    }
}

fn mode_label(mode: ComputeMode) -> &'static str {
    match mode {
        ComputeMode::Turbo => "Turbo",
        ComputeMode::Eco => "Eco",
        ComputeMode::Balanced => "Balanced",
        ComputeMode::Full => "Full",
    }
}

fn truncate(text: &str, max: usize) -> String {
    let cleaned = text.trim();
    let fallback = if cleaned.is_empty() { "New Tab" } else { cleaned };
    if fallback.chars().count() <= max {
        fallback.to_string()
    } else {
        let mut out: String = fallback.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn small(text: impl Into<String>) -> RichText {
    RichText::new(text).size(11.0).color(theme::MUTED)
}

pub fn draw(ctx: &egui::Context, state: &mut ChromeState) -> Option<ChromeAction> {
    let mut action: Option<ChromeAction> = None;
    let height = chrome_height(state);

    let frame = egui::Frame {
        fill: theme::BG,
        ..Default::default()
    };

    egui::TopBottomPanel::top("ferrox_chrome")
        .exact_height(height)
        .frame(frame)
        .show(ctx, |ui| {
            ui.add_space(PAD);

            // ---- Tabs row -------------------------------------------------
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), TABS_H),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    egui::ScrollArea::horizontal()
                        .id_salt("ferrox_tabs_scroll")
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for tab in &state.tabs {
                                    let selected = Some(tab.id) == state.active;
                                    let label = format!(
                                        "{}{}{}",
                                        if tab.loading { "◌ " } else { "" },
                                        if tab.state == TabState::Hibernated { "☾ " } else { "" },
                                        truncate(&tab.title, 24)
                                    );
                                    let text = RichText::new(label)
                                        .size(12.0)
                                        .color(if selected { theme::TEXT } else { theme::MUTED });
                                    let button = egui::Button::new(text)
                                        .fill(if selected { theme::PANEL_ALT } else { theme::PANEL })
                                        .stroke(if selected {
                                            egui::Stroke::new(1.0, theme::LINE)
                                        } else {
                                            egui::Stroke::NONE
                                        })
                                        .min_size(egui::vec2(118.0, 26.0));
                                    if ui.add(button).clicked() {
                                        action = Some(ChromeAction::ActivateTab(tab.id));
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new("×").size(14.0))
                                                .frame(false)
                                                .min_size(egui::vec2(18.0, 18.0)),
                                        )
                                        .clicked()
                                    {
                                        action = Some(ChromeAction::CloseTab(tab.id));
                                    }
                                    ui.add_space(4.0);
                                }
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("+").size(16.0))
                                            .frame(false)
                                            .min_size(egui::vec2(24.0, 24.0)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ChromeAction::NewTab);
                                }
                            });
                        });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if ui
                            .add(egui::Button::new(small("≡ Menu")).frame(false))
                            .clicked()
                        {
                            state.menu_open = !state.menu_open;
                        }
                        if state.incognito {
                            ui.label(
                                RichText::new("Incognito")
                                    .size(10.5)
                                    .color(theme::TEAL)
                                    .strong(),
                            );
                        }
                        ui.label(small(format!("{:.0} MB", state.memory_mb)));
                        if state.update_available {
                            ui.label(RichText::new("Update").size(10.5).color(theme::ACCENT));
                        }
                    });
                },
            );

            ui.add_space(3.0);

            // ---- Navigation row ------------------------------------------
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), NAV_H),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;

                    if ui
                        .add_enabled(state.can_back, egui::Button::new("←").frame(false))
                        .on_hover_text("Back")
                        .clicked()
                    {
                        action = Some(ChromeAction::Back);
                    }
                    if ui
                        .add_enabled(state.can_forward, egui::Button::new("→").frame(false))
                        .on_hover_text("Forward")
                        .clicked()
                    {
                        action = Some(ChromeAction::Forward);
                    }
                    let reload = if state.loading { "×" } else { "⟳" };
                    if ui
                        .add(egui::Button::new(reload).frame(false))
                        .on_hover_text("Reload")
                        .clicked()
                    {
                        action = Some(ChromeAction::Reload);
                    }
                    if ui
                        .add(egui::Button::new("⌂").frame(false))
                        .on_hover_text("Home")
                        .clicked()
                    {
                        action = Some(ChromeAction::Home);
                    }

                    let width = (ui.available_width() - 320.0).max(220.0);
                    let edit = TextEdit::singleline(&mut state.address)
                        .hint_text("Search, URL, !bang or =calculation")
                        .desired_width(width)
                        .margin(egui::vec2(10.0, 7.0));
                    let response = ui.add(edit);

                    if state.focus_address {
                        response.request_focus();
                        state.focus_address = false;
                    }
                    if response.changed() {
                        state.show_suggestions = !state.address.trim().is_empty();
                        state.selected = 0;
                        action = Some(ChromeAction::AddressChanged);
                    }
                    if response.has_focus() {
                        let (enter, escape, down, up) = ui.input(|i| {
                            (
                                i.key_pressed(Key::Enter),
                                i.key_pressed(Key::Escape),
                                i.key_pressed(Key::ArrowDown),
                                i.key_pressed(Key::ArrowUp),
                            )
                        });
                        if down && !state.suggestions.is_empty() {
                            state.selected = (state.selected + 1) % state.suggestions.len();
                            state.show_suggestions = true;
                        }
                        if up && !state.suggestions.is_empty() {
                            state.selected = state
                                .selected
                                .checked_sub(1)
                                .unwrap_or(state.suggestions.len() - 1);
                            state.show_suggestions = true;
                        }
                        if enter {
                            let target = if state.show_suggestions
                                && !state.suggestions.is_empty()
                            {
                                state
                                    .suggestions
                                    .get(state.selected)
                                    .map(|s| s.url.clone())
                                    .unwrap_or_else(|| state.address.clone())
                            } else {
                                state.address.clone()
                            };
                            state.show_suggestions = false;
                            response.surrender_focus();
                            action = Some(ChromeAction::Navigate(target));
                        }
                        if escape {
                            state.show_suggestions = false;
                            response.surrender_focus();
                        }
                    }

                    let shield_text = RichText::new(format!("Shield {}", state.blocked))
                        .size(11.0)
                        .color(if state.shield_enabled { theme::TEAL } else { theme::MUTED });
                    if ui
                        .add(
                            egui::Button::new(shield_text)
                                .fill(theme::PANEL)
                                .stroke(egui::Stroke::new(1.0, theme::LINE)),
                        )
                        .on_hover_text("Toggle tracker & ad shielding")
                        .clicked()
                    {
                        action = Some(ChromeAction::ToggleShield);
                    }

                    let mode = RichText::new(format!("◈ {}", mode_label(state.compute_mode)))
                        .size(11.0)
                        .color(compute_color(state.compute_mode));
                    if ui
                        .add(
                            egui::Button::new(mode)
                                .fill(theme::PANEL)
                                .stroke(egui::Stroke::new(1.0, theme::LINE)),
                        )
                        .on_hover_text("Cycle compute mode: Turbo → Eco → Balanced → Full")
                        .clicked()
                    {
                        action = Some(ChromeAction::CycleComputeMode);
                    }

                    if ui
                        .add(egui::Button::new("⚙").frame(false))
                        .on_hover_text("Settings")
                        .clicked()
                    {
                        action = Some(ChromeAction::OpenInternal("ferrox://settings".into()));
                    }
                },
            );

            // ---- Suggestions ---------------------------------------------
            if state.show_suggestions && !state.suggestions.is_empty() {
                ui.add_space(2.0);
                let count = state.suggestions.len().min(6);
                for index in 0..count {
                    let suggestion = state.suggestions[index].clone();
                    let selected = index == state.selected;
                    let bg = if selected { theme::PANEL_ALT } else { theme::BG };
                    let response = ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), SUGG_ROW),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            egui::Frame::NONE
                                .fill(bg)
                                .corner_radius(egui::CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(8, 2))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(
                                        RichText::new(suggestion.kind.to_ascii_uppercase())
                                            .size(9.5)
                                            .color(theme::ACCENT)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(truncate(&suggestion.title, 70))
                                            .size(12.0)
                                            .color(if selected { theme::TEXT } else { theme::MUTED }),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(truncate(&suggestion.url, 60))
                                                .size(10.0)
                                                .color(theme::MUTED),
                                        );
                                    });
                                });
                        },
                    )
                    .response;

                    if response.interact(Sense::click()).clicked() {
                        state.show_suggestions = false;
                        action = Some(ChromeAction::Navigate(suggestion.url));
                    }
                }
            }

            // ---- Menu ----------------------------------------------------
            if state.menu_open {
                ui.add_space(4.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), MENU_H),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let items = [
                            ("History", "ferrox://history"),
                            ("Bookmarks", "ferrox://bookmarks"),
                            ("Downloads", "ferrox://downloads"),
                            ("Profiler", "ferrox://profiler"),
                            ("Updates", "ferrox://updates"),
                            ("Settings", "ferrox://settings"),
                        ];
                        for (label, url) in items {
                            if ui.button(label).clicked() {
                                state.menu_open = false;
                                action = Some(ChromeAction::OpenInternal(url.to_string()));
                            }
                        }
                        ui.separator();
                        if ui
                            .button(if state.incognito {
                                "Leave Incognito"
                            } else {
                                "Incognito"
                            })
                            .clicked()
                        {
                            action = Some(ChromeAction::ToggleIncognito);
                        }
                        if ui.button("Check updates").clicked() {
                            action = Some(ChromeAction::CheckUpdates);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Quit").clicked() {
                                action = Some(ChromeAction::Quit);
                            }
                            if let Some(status) = &state.status {
                                ui.label(RichText::new(truncate(status, 70)).size(10.5).color(theme::MUTED));
                            }
                        });
                    },
                );
            }

            // Fill remaining height so the panel is exactly `height` tall.
            let used = ui.min_rect().height();
            if height > used {
                ui.add_space((height - used).max(0.0));
            }
        });

    action
}

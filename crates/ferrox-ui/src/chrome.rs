// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::theme;
use egui::{Align, Color32, Key, Layout, RichText, Sense, TextEdit};
use ferrox_core::{ComputeMode, TabId, TabState};

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
    /// Opens the find bar.
    Find,
    /// Closes the find bar and clears the highlights.
    FindClose,
    /// The find query changed and must be re-applied to the page.
    FindChanged(String),
    /// Moves to the next (`true`) or previous (`false`) match.
    FindNext(bool),
    /// Pins or unpins a tab so it never hibernates.
    TogglePin(TabId),
    /// Mutes or unmutes the media of a tab.
    ToggleMute(TabId),
    /// Opens the same URL in a new tab.
    DuplicateTab(TabId),
    /// Closes every tab except the given one.
    CloseOtherTabs(TabId),
    /// Opens the developer tools of the active tab.
    DevTools,
    /// Shows the source code of the active page.
    ViewSource,
    ZoomIn,
    ZoomOut,
    ZoomReset,
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
    /// Find-in-page bar state.
    pub find_open: bool,
    pub find_query: String,
    pub find_count: usize,
    pub find_index: usize,
    pub find_focus: bool,
    /// Tab that the user opened the action row for.
    pub tab_menu: Option<TabId>,
    /// Current zoom level of the active tab in percent.
    pub zoom_percent: u32,
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
            find_open: false,
            find_query: String::new(),
            find_count: 0,
            find_index: 0,
            find_focus: false,
            tab_menu: None,
            zoom_percent: 100,
        }
    }
}

const TABS_H: f32 = 30.0;
const NAV_H: f32 = 36.0;
const SUGG_ROW: f32 = 25.0;
const MENU_H: f32 = 30.0;
const FIND_H: f32 = 30.0;
const TABMENU_H: f32 = 30.0;
const PAD: f32 = 7.0;
/// Space that is reserved on the right side of the tab strip.
const INDICATOR_WIDTH: f32 = 330.0;

pub fn chrome_height(state: &ChromeState) -> f32 {
    let suggestions = if state.show_suggestions && !state.suggestions.is_empty() {
        state.suggestions.len().min(6) as f32 * SUGG_ROW + 4.0
    } else {
        0.0
    };
    let menu = if state.menu_open { MENU_H + 4.0 } else { 0.0 };
    let find = if state.find_open { FIND_H + 3.0 } else { 0.0 };
    let tab_menu = if state.tab_menu.is_some() {
        TABMENU_H + 3.0
    } else {
        0.0
    };
    PAD + TABS_H + 3.0 + NAV_H + find + tab_menu + suggestions + menu + PAD
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
    let fallback = if cleaned.is_empty() {
        "New Tab"
    } else {
        cleaned
    };
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
        stroke: egui::Stroke::new(1.0, theme::LINE),
        ..Default::default()
    };

    use egui::{Id, LayerId, UiBuilder};
    let mut root_ui = egui::Ui::new(
        ctx.clone(),
        Id::unique("ferrox_chrome_root"),
        UiBuilder::new()
            .layer_id(LayerId::new(egui::Order::Middle, Id::unique("chrome")))
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::top("ferrox_chrome")
        .exact_size(height)
        .frame(frame)
        .show(&mut root_ui, |ui| {
            ui.add_space(PAD);

            // ---- Tabs row -------------------------------------------------
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), TABS_H),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    // The strip must not eat the space of the status indicators.
                    let tabs_width = (ui.available_width() - INDICATOR_WIDTH).max(120.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(tabs_width, TABS_H),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            egui::ScrollArea::horizontal()
                                .id_salt("ferrox_tabs_scroll")
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                )
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        let tabs = state.tabs.clone();
                                        for tab in &tabs {
                                            let selected = Some(tab.id) == state.active;
                                            let label = format!(
                                                "{}{}{}{}{}",
                                                if tab.loading { "◌ " } else { "" },
                                                if tab.state == TabState::Hibernated {
                                                    "☾ "
                                                } else {
                                                    ""
                                                },
                                                if tab.pinned { "📌 " } else { "" },
                                                if tab.muted { "🔇 " } else { "" },
                                                truncate(&tab.title, 24)
                                            );
                                            let text = RichText::new(label).size(12.0).color(
                                                if selected { theme::TEXT } else { theme::MUTED },
                                            );
                                            let button = egui::Button::new(text)
                                                .fill(if selected {
                                                    theme::PANEL_ALT
                                                } else {
                                                    theme::PANEL
                                                })
                                                .stroke(if selected {
                                                    egui::Stroke::new(1.0, theme::ACCENT)
                                                } else {
                                                    egui::Stroke::NONE
                                                })
                                                .min_size(egui::vec2(118.0, 26.0));
                                            let response = ui.add(button);
                                            if response.clicked() {
                                                action = Some(ChromeAction::ActivateTab(tab.id));
                                            } else if response.secondary_clicked() {
                                                state.tab_menu = Some(tab.id);
                                            } else if response.middle_clicked() {
                                                action = Some(ChromeAction::CloseTab(tab.id));
                                            }
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("×").size(14.0),
                                                    )
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
                        },
                    );

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
                        if state.zoom_percent != 100 {
                            ui.label(
                                RichText::new(format!("{}%", state.zoom_percent))
                                    .size(10.5)
                                    .color(theme::MUTED),
                            );
                        }
                        ui.label(small(format!("{:.0} MB", state.memory_mb)));
                        if state.update_available {
                            ui.label(RichText::new("Update").size(10.5).color(theme::ACCENT));
                        }
                        if let Some(status) = &state.status {
                            ui.label(
                                RichText::new(truncate(status, 40))
                                    .size(10.5)
                                    .color(theme::ACCENT_SOFT),
                            );
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
                    let reload = if state.loading { "◌" } else { "⟳" };
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
                            let selected = state.suggestions.get(state.selected).cloned();
                            let target = if state.show_suggestions && !state.suggestions.is_empty()
                            {
                                selected
                                    .as_ref()
                                    .map(|s| s.url.clone())
                                    .unwrap_or_else(|| state.address.clone())
                            } else {
                                state.address.clone()
                            };
                            state.show_suggestions = false;
                            response.surrender_focus();
                            // A calculation is copied into the address bar
                            // instead of being opened as a search.
                            if selected.map(|s| s.kind == "calc").unwrap_or(false) {
                                state.address = target;
                            } else {
                                action = Some(ChromeAction::Navigate(target));
                            }
                        }
                        if escape {
                            state.show_suggestions = false;
                            response.surrender_focus();
                        }
                    }

                    let shield_text = RichText::new(format!("Shield {}", state.blocked))
                        .size(11.0)
                        .color(if state.shield_enabled {
                            theme::TEAL
                        } else {
                            theme::MUTED
                        });
                    if ui
                        .add(
                            egui::Button::new(shield_text)
                                .fill(if state.shield_enabled {
                                    Color32::from_rgb(15, 42, 42)
                                } else {
                                    theme::PANEL
                                })
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

            // ---- Find in page ---------------------------------------------
            if state.find_open {
                ui.add_space(3.0);
                let mut query = state.find_query.clone();
                let mut next_clicked = false;
                let mut prev_clicked = false;
                let mut close_clicked = false;
                let mut changed = false;
                let mut enter = false;
                let mut escape = false;
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), FIND_H),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(RichText::new("\u{1f50e}").size(12.0));
                        let edit = TextEdit::singleline(&mut query)
                            .hint_text("Find in page")
                            .desired_width(240.0)
                            .margin(egui::vec2(10.0, 5.0));
                        let response = ui.add(edit);
                        if state.find_focus {
                            response.request_focus();
                            state.find_focus = false;
                        }
                        if response.has_focus() {
                            (enter, escape) = ui
                                .input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
                        }
                        changed = response.changed();
                        let count = state.find_count;
                        let index = state.find_index;
                        ui.label(
                            RichText::new(if count == 0 {
                                "no results".to_string()
                            } else {
                                format!("{index} of {count}")
                            })
                            .size(11.0)
                            .color(theme::MUTED),
                        );
                        prev_clicked = ui
                            .add(
                                egui::Button::new("\u{2191}")
                                    .frame(false)
                                    .min_size(egui::vec2(24.0, 22.0)),
                            )
                            .on_hover_text("Previous match")
                            .clicked();
                        next_clicked = ui
                            .add(
                                egui::Button::new("\u{2193}")
                                    .frame(false)
                                    .min_size(egui::vec2(24.0, 22.0)),
                            )
                            .on_hover_text("Next match")
                            .clicked();
                        close_clicked = ui
                            .add(
                                egui::Button::new("\u{2715}")
                                    .frame(false)
                                    .min_size(egui::vec2(24.0, 22.0)),
                            )
                            .on_hover_text("Close find bar")
                            .clicked();
                    },
                );

                state.find_query = query;
                if changed {
                    state.find_count = 0;
                    state.find_index = 0;
                    action = Some(ChromeAction::FindChanged(state.find_query.clone()));
                }
                if next_clicked || enter {
                    action = Some(ChromeAction::FindNext(true));
                }
                if prev_clicked {
                    action = Some(ChromeAction::FindNext(false));
                }
                if close_clicked || escape {
                    action = Some(ChromeAction::FindClose);
                }
            }

            // ---- Tab actions ----------------------------------------------
            if let Some(tab_id) = state.tab_menu {
                let tabs = state.tabs.clone();
                let tab = tabs.iter().find(|t| t.id == tab_id);
                let (pinned, muted, title) = tab
                    .map(|t| (t.pinned, t.muted, truncate(&t.title, 32)))
                    .unwrap_or((false, false, String::new()));
                let mut pin = false;
                let mut mute = false;
                let mut duplicate = false;
                let mut close_others = false;
                let mut close = false;
                let mut dismiss = false;
                ui.add_space(3.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), TABMENU_H),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(
                            RichText::new(format!("Tab: {title}"))
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                        pin = ui
                            .add(egui::Button::new(if pinned { "Unpin" } else { "Pin" }))
                            .clicked();
                        mute = ui
                            .add(egui::Button::new(if muted { "Unmute" } else { "Mute" }))
                            .clicked();
                        duplicate = ui.add(egui::Button::new("Duplicate")).clicked();
                        close_others = ui.add(egui::Button::new("Close others")).clicked();
                        close = ui.add(egui::Button::new("Close")).clicked();
                        dismiss = ui.add(egui::Button::new("Dismiss")).clicked();
                    },
                );

                if dismiss {
                    state.tab_menu = None;
                }
                if pin {
                    state.tab_menu = None;
                    action = Some(ChromeAction::TogglePin(tab_id));
                }
                if mute {
                    state.tab_menu = None;
                    action = Some(ChromeAction::ToggleMute(tab_id));
                }
                if duplicate {
                    state.tab_menu = None;
                    action = Some(ChromeAction::DuplicateTab(tab_id));
                }
                if close_others {
                    state.tab_menu = None;
                    action = Some(ChromeAction::CloseOtherTabs(tab_id));
                }
                if close {
                    state.tab_menu = None;
                    action = Some(ChromeAction::CloseTab(tab_id));
                }
            }

            // ---- Suggestions ---------------------------------------------
            if state.show_suggestions && !state.suggestions.is_empty() {
                ui.add_space(2.0);
                let count = state.suggestions.len().min(6);
                for index in 0..count {
                    let suggestion = state.suggestions[index].clone();
                    let selected = index == state.selected;
                    let bg = if selected {
                        theme::PANEL_ALT
                    } else {
                        theme::BG
                    };
                    let response = ui
                        .allocate_ui_with_layout(
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
                                                .color(if selected {
                                                    theme::TEXT
                                                } else {
                                                    theme::MUTED
                                                }),
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(truncate(&suggestion.url, 60))
                                                        .size(10.0)
                                                        .color(theme::MUTED),
                                                );
                                            },
                                        );
                                    });
                            },
                        )
                        .response;

                    if response.interact(Sense::click()).clicked() {
                        state.show_suggestions = false;
                        if suggestion.kind == "calc" {
                            // Calculations are copied into the address bar.
                            state.address = suggestion.url.clone();
                        } else {
                            action = Some(ChromeAction::Navigate(suggestion.url));
                        }
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
                        egui::ScrollArea::horizontal()
                            .id_salt("ferrox_menu_scroll")
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
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
                                            action =
                                                Some(ChromeAction::OpenInternal(url.to_string()));
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
                                        state.menu_open = false;
                                        action = Some(ChromeAction::ToggleIncognito);
                                    }
                                    if ui
                                        .add(egui::Button::new("\u{1f50d}").frame(false))
                                        .on_hover_text("Find in page (Ctrl+F)")
                                        .clicked()
                                    {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::Find);
                                    }
                                    if ui
                                        .add(egui::Button::new("\u{2212}").frame(false))
                                        .on_hover_text("Zoom out (Ctrl+-)")
                                        .clicked()
                                    {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::ZoomOut);
                                    }
                                    if ui
                                        .add(egui::Button::new("+").frame(false))
                                        .on_hover_text("Zoom in (Ctrl+=)")
                                        .clicked()
                                    {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::ZoomIn);
                                    }
                                    if ui
                                        .add(egui::Button::new("1:1").frame(false))
                                        .on_hover_text("Reset zoom (Ctrl+0)")
                                        .clicked()
                                    {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::ZoomReset);
                                    }
                                    if ui
                                        .add(egui::Button::new("</>").frame(false))
                                        .on_hover_text("View the source of this page")
                                        .clicked()
                                    {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::ViewSource);
                                    }
                                    if ui
                                        .add(egui::Button::new("Dev").frame(false))
                                        .on_hover_text("Open the developer tools")
                                        .clicked()
                                    {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::DevTools);
                                    }
                                    if ui.button("Check updates").clicked() {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::CheckUpdates);
                                    }
                                    if ui.button("Quit").clicked() {
                                        state.menu_open = false;
                                        action = Some(ChromeAction::Quit);
                                    }
                                });
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

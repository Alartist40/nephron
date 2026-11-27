//! A2UI-inspired declarative generative UI dashboard renderer for Nephron responses.

use crate::studio::techart_tag;
use eframe::egui::{self, Color32, Frame, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiMetric {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub delta: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiKeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiSection {
    pub title: String,
    #[serde(default)]
    pub items: Vec<A2uiKeyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiCheckItem {
    pub label: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiTable {
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiAction {
    pub label: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct A2uiSurface {
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub badge: Option<String>,
    #[serde(default)]
    pub metrics: Vec<A2uiMetric>,
    #[serde(default)]
    pub sections: Vec<A2uiSection>,
    #[serde(default)]
    pub checklist: Vec<A2uiCheckItem>,
    #[serde(default)]
    pub table: Option<A2uiTable>,
    #[serde(default)]
    pub actions: Vec<A2uiAction>,
}

fn tone_color(tone: Option<&str>) -> Color32 {
    match tone.unwrap_or("sky") {
        "emerald" | "green" => Color32::from_rgb(110, 231, 183),
        "amber" | "yellow" => Color32::from_rgb(252, 211, 77),
        "rose" | "red" => Color32::from_rgb(253, 164, 175),
        "purple" | "indigo" => Color32::from_rgb(192, 132, 252),
        _ => Color32::from_rgb(125, 211, 252),
    }
}

impl A2uiSurface {
    pub fn parse_from_text(raw: &str) -> Option<Self> {
        for fence in ["```a2ui", "```json"] {
            if let Some(start) = raw.find(fence) {
                let rest = &raw[start + fence.len()..];
                if let Some(end) = rest.find("```") {
                    if let Ok(surface) = serde_json::from_str::<A2uiSurface>(rest[..end].trim()) {
                        if !surface.title.trim().is_empty() {
                            return Some(surface);
                        }
                    }
                }
            }
        }
        let trimmed = raw.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(surface) = serde_json::from_str::<A2uiSurface>(trimmed) {
                if !surface.title.trim().is_empty() {
                    return Some(surface);
                }
            }
        }
        None
    }

    pub fn from_markdown_heuristic(raw: &str) -> Option<Self> {
        let mut metrics = Vec::new();
        let mut kv_items = Vec::new();
        let mut checklist = Vec::new();
        let mut title = String::new();
        for line in raw.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if title.is_empty() && l.starts_with('#') {
                title = l.trim_start_matches('#').trim().to_string();
                continue;
            }
            if let Some(rest) = l.strip_prefix("- [ ] ").or_else(|| l.strip_prefix("* [ ] ")) {
                checklist.push(A2uiCheckItem { label: rest.trim().to_string(), done: false });
            } else if let Some(rest) = l.strip_prefix("- [x] ").or_else(|| l.strip_prefix("- [X] ")) {
                checklist.push(A2uiCheckItem { label: rest.trim().to_string(), done: true });
            } else if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                if let Some((k, v)) = rest.split_once(':') {
                    let k = k.trim().trim_matches('*');
                    let v = v.trim();
                    if !k.is_empty() && !v.is_empty() && k.len() <= 28 {
                        if v.len() <= 18 && metrics.len() < 4 {
                            metrics.push(A2uiMetric {
                                label: k.to_string(),
                                value: v.to_string(),
                                delta: None,
                                tone: Some("sky".into()),
                            });
                        } else {
                            kv_items.push(A2uiKeyValue {
                                key: k.to_string(),
                                value: v.to_string(),
                            });
                        }
                    }
                }
            }
        }
        if metrics.len() >= 2 || kv_items.len() >= 2 || checklist.len() >= 2 {
            Some(A2uiSurface {
                title: if title.is_empty() { "Structured Dashboard".into() } else { title },
                subtitle: None,
                badge: Some("A2UI".into()),
                metrics,
                sections: if kv_items.is_empty() {
                    Vec::new()
                } else {
                    vec![A2uiSection { title: "Breakdown".into(), items: kv_items }]
                },
                checklist,
                table: None,
                actions: Vec::new(),
            })
        } else {
            None
        }
    }
}

pub fn draw_a2ui_surface(ui: &mut egui::Ui, width: f32, surface: &mut A2uiSurface) -> Option<A2uiAction> {
    let mut triggered: Option<A2uiAction> = None;
    Frame::NONE
        .fill(Color32::from_rgba_unmultiplied(12, 18, 32, 225))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(125, 211, 252, 48)))
        .corner_radius(18.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            let inner_w = (width - 28.0).max(180.0);
            ui.vertical(|ui| {
                ui.set_width(inner_w);
                ui.spacing_mut().item_spacing.y = 8.0;

                // Header row
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&surface.title).size(13.5).strong().color(Color32::WHITE));
                    if let Some(badge) = &surface.badge {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            techart_tag(ui, badge, Color32::from_rgb(125, 211, 252));
                        });
                    }
                });
                if let Some(sub) = &surface.subtitle {
                    ui.add(egui::Label::new(egui::RichText::new(sub).size(11.5).color(Color32::from_rgb(148, 163, 184))).wrap());
                }

                // KPI Metrics grid
                if !surface.metrics.is_empty() {
                    let cols = surface.metrics.len().clamp(1, 3);
                    let card_w = ((inner_w - (cols - 1) as f32 * 8.0) / cols as f32).max(76.0);
                    for row in surface.metrics.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for m in row {
                                let col = tone_color(m.tone.as_deref());
                                Frame::NONE
                                    .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 9))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 55)))
                                    .corner_radius(12.0)
                                    .inner_margin(10.0)
                                    .show(ui, |ui| {
                                        ui.set_min_width(card_w - 20.0);
                                        ui.set_max_width(card_w - 20.0);
                                        ui.vertical(|ui| {
                                            ui.small(egui::RichText::new(&m.label).color(Color32::from_rgb(148, 163, 184)));
                                            ui.label(egui::RichText::new(&m.value).size(15.0).strong().color(col));
                                            if let Some(d) = &m.delta {
                                                ui.small(egui::RichText::new(d).monospace().color(Color32::from_rgb(203, 213, 225)));
                                            }
                                        });
                                    });
                            }
                        });
                    }
                }

                // Key-Value sections
                for sec in &surface.sections {
                    Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 6))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 18)))
                        .corner_radius(12.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.set_width(inner_w - 20.0);
                            ui.vertical(|ui| {
                                ui.small(egui::RichText::new(&sec.title).strong().color(Color32::from_rgb(186, 230, 253)));
                                for kv in &sec.items {
                                    ui.horizontal(|ui| {
                                        ui.small(egui::RichText::new(format!("{}:", kv.key)).strong().color(Color32::from_rgb(148, 163, 184)));
                                        ui.add(egui::Label::new(egui::RichText::new(&kv.value).size(11.5).color(Color32::from_rgb(241, 245, 249))).wrap());
                                    });
                                }
                            });
                        });
                }

                // Comparison table
                if let Some(tbl) = &surface.table {
                    if !tbl.headers.is_empty() {
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(6, 10, 18, 180))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 18)))
                            .corner_radius(12.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                let cols = tbl.headers.len().max(1);
                                let cw = ((inner_w - 24.0) / cols as f32).max(52.0);
                                ui.horizontal(|ui| {
                                    for h in &tbl.headers {
                                        ui.add_sized([cw, 18.0], egui::Label::new(egui::RichText::new(h).size(10.5).strong().color(Color32::from_rgb(125, 211, 252))));
                                    }
                                });
                                ui.separator();
                                for r in &tbl.rows {
                                    ui.horizontal(|ui| {
                                        for cell in r.iter().take(cols) {
                                            ui.add_sized([cw, 18.0], egui::Label::new(egui::RichText::new(cell).size(10.5).color(Color32::from_rgb(226, 232, 240))));
                                        }
                                    });
                                }
                            });
                    }
                }

                // Interactive checklist
                if !surface.checklist.is_empty() {
                    ui.vertical(|ui| {
                        for item in &mut surface.checklist {
                            ui.checkbox(&mut item.done, egui::RichText::new(&item.label).size(11.5));
                        }
                    });
                }

                // Action pills
                if !surface.actions.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for act in &surface.actions {
                            let btn = egui::Button::new(egui::RichText::new(&act.label).size(11.0).color(Color32::from_rgb(186, 230, 253)))
                                .fill(Color32::from_rgba_unmultiplied(56, 189, 248, 32))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 85)))
                                .corner_radius(99.0);
                            if ui.add(btn).clicked() {
                                triggered = Some(act.clone());
                            }
                        }
                    });
                }
            });
        });
    triggered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_a2ui_dashboard() {
        let raw = "Here is your overview:\n```a2ui\n{\"title\":\"Orange Pi 6 Health\",\"badge\":\"LIVE\",\"metrics\":[{\"label\":\"RAM\",\"value\":\"6.4 GiB\",\"tone\":\"emerald\"}],\"actions\":[{\"label\":\"Refresh\",\"kind\":\"prompt\",\"prompt\":\"Check RAM\"}]}\n```";
        let s = A2uiSurface::parse_from_text(raw).unwrap();
        assert_eq!(s.title, "Orange Pi 6 Health");
        assert_eq!(s.metrics.len(), 1);
        assert_eq!(s.actions.len(), 1);
    }

    #[test]
    fn converts_markdown_bullets_to_a2ui_dashboard() {
        let md = "# Study Plan\n- Focus: Rust SDF CSG\n- Time: 25m\n- [ ] Build pyramid mesh\n- [x] Export STL";
        let s = A2uiSurface::from_markdown_heuristic(md).unwrap();
        assert_eq!(s.title, "Study Plan");
        assert_eq!(s.metrics.len(), 2);
        assert_eq!(s.checklist.len(), 2);
    }
}

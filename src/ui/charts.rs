//! Gráficos desenhados com o painter do egui — sem dependência de charting.

use eframe::egui::{self, Align2, Color32, FontId, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use granaflow::model::fmt_brl;

use super::ACCENT;

/// Série de linha com preenchimento suave e linha de zero.
pub fn line(ui: &mut Ui, points: &[(String, i64)], height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let text = ui.visuals().weak_text_color();
    if points.is_empty() {
        return;
    }
    let min = points.iter().map(|p| p.1).min().unwrap_or(0).min(0) as f32;
    let max = points.iter().map(|p| p.1).max().unwrap_or(0).max(0) as f32;
    let span = (max - min).max(1.0);
    let inner = rect.shrink2(Vec2::new(6.0, 14.0));
    let n = points.len();
    let x = |i: usize| {
        if n == 1 {
            inner.center().x
        } else {
            inner.left() + inner.width() * (i as f32 / (n - 1) as f32)
        }
    };
    let y = |v: i64| inner.bottom() - inner.height() * ((v as f32 - min) / span);

    let zero_y = y(0);
    painter.line_segment(
        [
            egui::pos2(inner.left(), zero_y),
            egui::pos2(inner.right(), zero_y),
        ],
        Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );

    let fill = Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), 26);
    for (i, p) in points.iter().enumerate() {
        let px = x(i);
        let py = y(p.1);
        let top = py.min(zero_y);
        let bot = py.max(zero_y);
        painter.rect_filled(
            Rect::from_min_max(egui::pos2(px - 3.0, top), egui::pos2(px + 3.0, bot)),
            Rounding::ZERO,
            fill,
        );
    }

    let pts: Vec<egui::Pos2> = points
        .iter()
        .enumerate()
        .map(|(i, p)| egui::pos2(x(i), y(p.1)))
        .collect();
    painter.add(egui::Shape::line(pts.clone(), Stroke::new(2.0_f32, ACCENT)));
    for p in &pts {
        painter.circle_filled(*p, 3.0, ACCENT);
    }
    if n <= 8 {
        for (i, (label, _)) in points.iter().enumerate() {
            let short = label.get(5..).unwrap_or(label);
            painter.text(
                egui::pos2(x(i), inner.bottom() + 4.0),
                Align2::CENTER_TOP,
                short,
                FontId::proportional(10.0),
                text,
            );
        }
    }
}

/// Barras horizontais com rótulo e valor.
pub fn bars(ui: &mut Ui, items: &[(String, i64, Color32)]) {
    if items.is_empty() {
        ui.weak("Sem dados neste mês.");
        return;
    }
    let max = items.iter().map(|i| i.1).max().unwrap_or(1).max(1);
    let width = ui.available_width();
    for (name, value, color) in items {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::hover());
        let painter = ui.painter_at(rect);
        let text_color = ui.visuals().text_color();
        let label_w = 118.0;
        painter.text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(12.5),
            text_color,
        );
        let track = Rect::from_min_max(
            egui::pos2(rect.left() + label_w, rect.center().y - 5.0),
            egui::pos2((rect.right() - 88.0).max(rect.left() + label_w + 4.0), rect.center().y + 5.0),
        );
        painter.rect_filled(
            track,
            Rounding::same(4.0),
            ui.visuals().widgets.noninteractive.bg_fill,
        );
        let ratio = (*value as f32 / max as f32).clamp(0.02, 1.0);
        painter.rect_filled(
            Rect::from_min_size(track.min, Vec2::new(track.width() * ratio, track.height())),
            Rounding::same(4.0),
            *color,
        );
        painter.text(
            rect.right_center(),
            Align2::RIGHT_CENTER,
            fmt_brl(*value),
            FontId::proportional(12.0),
            text_color,
        );
    }
}


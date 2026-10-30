use super::*;
use egui::{Align, Align2, FontId, Rect, Sense, pos2};

pub(crate) fn update_device_row(ui: &mut egui::Ui, alias: &str, version: &str) {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .corner_radius(theme::CONTROL_RADIUS)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
            let painter = ui.painter_at(rect);
            let screen = Rect::from_center_size(
                pos2(rect.left() + 9.0, rect.center().y - 2.0),
                vec2(16.0, 11.0),
            );
            painter.rect_stroke(
                screen,
                2,
                Stroke::new(1.2, theme::MUTED),
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    screen.center_bottom(),
                    screen.center_bottom() + vec2(0.0, 3.0),
                ],
                Stroke::new(1.2, theme::MUTED),
            );
            painter.line_segment(
                [
                    screen.center_bottom() + vec2(-4.0, 3.0),
                    screen.center_bottom() + vec2(4.0, 3.0),
                ],
                Stroke::new(1.2, theme::MUTED),
            );
            let version_width = painter
                .layout_no_wrap(
                    version.to_owned(),
                    FontId::proportional(theme::SMALL),
                    theme::MUTED,
                )
                .size()
                .x
                .min(120.0);
            let name_rect = Rect::from_min_max(
                rect.min + vec2(30.0, 0.0),
                pos2(
                    (rect.right() - version_width - 16.0).max(rect.left() + 30.0),
                    rect.bottom(),
                ),
            );
            let mut name_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(name_rect)
                    .layout(egui::Layout::left_to_right(Align::Center)),
            );
            let name_response = name_ui
                .add(egui::Label::new(RichText::new(alias).size(theme::COMPACT_TEXT)).truncate());
            if painter
                .layout_no_wrap(
                    alias.to_owned(),
                    FontId::proportional(theme::COMPACT_TEXT),
                    theme::TEXT,
                )
                .size()
                .x
                > name_rect.width()
            {
                name_response.on_hover_text(alias);
            }
            let version_rect =
                Rect::from_min_max(pos2(rect.right() - version_width, rect.top()), rect.max);
            let mut version_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(version_rect)
                    .layout(egui::Layout::right_to_left(Align::Center)),
            );
            version_ui.add(
                egui::Label::new(
                    RichText::new(version)
                        .size(theme::SMALL)
                        .color(theme::MUTED),
                )
                .truncate(),
            );
        });
}

pub(crate) fn update_actions(
    ui: &mut egui::Ui,
    primary_label: Option<&str>,
    secondary_label: Option<(&str, &str)>,
) -> (bool, bool) {
    super::dialogs::dialog_actions_with_hint(
        ui,
        primary_label.map(DialogAction::new),
        secondary_label,
    )
}

pub(crate) fn update_prepared_notice(ctx: &egui::Context) -> bool {
    let mut postpone = false;
    egui::Area::new(egui::Id::unique("remote-update-prepared"))
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            dialog_frame()
                .inner_margin(egui::Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.set_width(theme::REMOTE_UPGRADE_WIDTH);
                    ui.horizontal(|ui| {
                        dialog_icon(ui, DialogIcon::Ready, 32.0);
                        ui.add_space(2.0);
                        ui.label(RichText::new("被控端更新已准备就绪").size(theme::COMPACT_TEXT));
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            postpone = ui.add(secondary("暂不安装")).clicked();
                        });
                    });
                });
        });
    postpone
}

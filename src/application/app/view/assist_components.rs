use super::*;
use crate::account::assist::SavedDevice;

pub(super) fn header(
    ui: &mut egui::Ui,
    title: &str,
    description: Option<&str>,
    right: impl FnOnce(&mut egui::Ui),
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::Frame::new()
        .inner_margin(theme::ASSIST_HEADER_MARGIN)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                ui.set_min_height(if description.is_some() {
                    22.0
                } else {
                    theme::CONTROL_HEIGHT
                });
                ui.label(RichText::new(title).size(theme::SECTION).strong());
                ui.with_layout(egui::Layout::right_to_left(Align::Center), right);
            });
            if let Some(description) = description {
                ui.label(
                    RichText::new(description)
                        .size(theme::COMPACT_TEXT)
                        .color(MUTED),
                );
            }
        });
}

pub(super) fn glyph(ui: &mut egui::Ui, icon: Icon, label: &str, color: Color32) -> egui::Response {
    let (_, response) = crate::ui::controls::icon_button_area(ui, theme::CONTROL_HEIGHT);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label)
    });
    paint_icon(
        ui.painter(),
        response.rect,
        icon,
        if !ui.is_enabled() {
            theme::DISABLED
        } else if color == MUTED && (response.hovered() || response.has_focus()) {
            TEXT
        } else {
            color
        },
    );
    response
}

pub(super) fn grouped_id(id: &str) -> String {
    if id.len() == 9 && id.bytes().all(|b| b.is_ascii_digit()) {
        format!("{} {} {}", &id[..3], &id[3..6], &id[6..])
    } else {
        id.to_owned()
    }
}

pub(super) fn copy_value(
    ui: &mut egui::Ui,
    salt: &str,
    size: egui::Vec2,
    display: RichText,
    value: Option<&str>,
    feedback: &str,
) {
    let copied_id = ui.scope_id().with(("assist-copy", salt));
    // Keep only a short-lived value fingerprint, never the credential text.
    let fingerprint = egui::Id::unique(value);
    let copied = ui
        .ctx()
        .data(|data| data.get_temp::<(Instant, egui::Id)>(copied_id));
    let copied =
        copied.filter(|(at, which)| *which == fingerprint && at.elapsed() < Duration::from_secs(2));
    if copied.is_none() {
        ui.ctx()
            .data_mut(|data| data.remove::<(Instant, egui::Id)>(copied_id));
    }
    let label = if copied.is_some() {
        RichText::new(feedback).size(theme::BODY).color(GREEN)
    } else {
        display
    };
    let response = ui
        .add_enabled_ui(value.is_some(), |ui| {
            crate::ui::controls::value_button(ui, size, label)
        })
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled() && value.is_some(),
            feedback.trim_start_matches("已"),
        )
    });
    if response.clicked()
        && let Some(value) = value
    {
        ui.ctx().copy_text(value.to_owned());
        ui.ctx()
            .data_mut(|data| data.insert_temp(copied_id, (Instant::now(), fingerprint)));
        ui.ctx().request_repaint();
    }
    if let Some((at, _)) = copied {
        ui.ctx()
            .request_repaint_after(Duration::from_secs(2).saturating_sub(at.elapsed()));
    }
}

pub(super) fn connect_inputs(
    ui: &mut egui::Ui,
    id: &mut String,
    code: &mut String,
    enabled: bool,
) -> (bool, bool) {
    let mut changed = false;
    let mut connect = false;
    ui.horizontal(|ui| {
        text_slot(
            ui,
            theme::ASSIST_INPUT_WIDTH,
            20.0,
            RichText::new("伙伴的设备 ID").color(MUTED),
        );
        text_slot(
            ui,
            theme::ASSIST_INPUT_WIDTH,
            20.0,
            RichText::new("验证码 / 密码（可选）").color(MUTED),
        );
    });
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            let input = ui.add_sized(
                [theme::ASSIST_INPUT_WIDTH, theme::CONTROL_HEIGHT],
                singleline_input(id)
                    .hint_text("输入9位设备 ID")
                    .char_limit(24),
            );
            if input.changed() {
                code.clear();
                changed = true;
            }
            let password = ui.add_sized(
                [theme::ASSIST_INPUT_WIDTH, theme::CONTROL_HEIGHT],
                singleline_input(code)
                    .hint_text("输入验证码或自定义密码")
                    .char_limit(256),
            );
            connect = ui
                .add_sized(
                    [theme::ASSIST_SHARE_WIDTH, theme::CONTROL_HEIGHT],
                    login_button("连接").fill(BLUE).stroke(Stroke::NONE),
                )
                .clicked()
                || ((input.has_focus() || password.has_focus())
                    && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        });
    });
    (changed, connect && enabled)
}

pub(super) fn text_slot(ui: &mut egui::Ui, width: f32, height: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        vec2(width, height),
        egui::Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_size(vec2(width, height));
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

pub(super) fn recent_grid(
    ui: &mut egui::Ui,
    items: &[SavedDevice],
    enabled: bool,
) -> Option<(usize, RecentAction)> {
    let gap = ui.spacing().item_spacing.x;
    let columns = ((ui.available_width() + gap) / (theme::ASSIST_RECENT_SIZE.x + gap))
        .floor()
        .max(1.0) as usize;
    let mut action = None;
    for (row, chunk) in items.chunks(columns).enumerate() {
        ui.horizontal(|ui| {
            for (column, item) in chunk.iter().enumerate() {
                ui.push_id(&item.publisher_device_id, |ui| {
                    if let Some(value) = recent_item(ui, item, enabled && item.validate().is_ok()) {
                        action = Some((row * columns + column, value));
                    }
                });
            }
        });
    }
    action
}

pub(super) enum RecentAction {
    Connect,
    Favorite,
    Remove,
}

pub(super) fn recent_item(
    ui: &mut egui::Ui,
    item: &SavedDevice,
    enabled: bool,
) -> Option<RecentAction> {
    let (rect, _) = ui.allocate_exact_size(theme::ASSIST_RECENT_SIZE, Sense::hover());
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(6.0))
            .layout(egui::Layout::left_to_right(Align::Center)),
    );
    if !enabled {
        row.disable();
    }
    row.spacing_mut().item_spacing.x = 2.0;
    ui.painter().rect_filled(rect, theme::CONTROL_RADIUS, BG);
    ui.painter().rect_stroke(
        rect,
        theme::CONTROL_RADIUS,
        Stroke::new(1.0, LINE),
        egui::StrokeKind::Inside,
    );
    let mut action = None;
    if glyph(
        &mut row,
        Icon::Star,
        if item.is_favorite {
            "取消收藏"
        } else {
            "收藏设备"
        },
        if item.is_favorite { AMBER } else { MUTED },
    )
    .clicked()
    {
        action = Some(RecentAction::Favorite);
    }
    let width = row.available_width() - theme::CONTROL_HEIGHT - 2.0;
    let response = row.add_enabled(
        enabled,
        egui::Button::new("")
            .frame(false)
            .min_size(vec2(width, rect.height() - 12.0)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            enabled,
            format!("连接 {}", item.title()),
        )
    });
    let mut text = row.new_child(
        egui::UiBuilder::new()
            .max_rect(response.rect)
            .layout(egui::Layout::top_down(Align::Min)),
    );
    text.spacing_mut().item_spacing.y = 0.0;
    if item.remark.trim().is_empty() {
        text_slot(
            &mut text,
            width,
            36.0,
            RichText::new(grouped_id(&item.connect_id)).color(TEXT),
        );
    } else {
        text_slot(
            &mut text,
            width,
            20.0,
            RichText::new(item.title()).color(TEXT),
        );
        text_slot(
            &mut text,
            width,
            16.0,
            RichText::new(grouped_id(&item.connect_id))
                .size(theme::SMALL)
                .color(MUTED),
        );
    }
    if response.clicked() {
        action = Some(RecentAction::Connect);
    }
    if glyph(&mut row, Icon::Close, "移除记录", MUTED).clicked() {
        action = Some(RecentAction::Remove);
    }
    action
}

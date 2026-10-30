//! Paint-only status icons: no egui area, response, tooltip or mouse hit region.
use crate::diagnostics::performance::health::{Assessment, Metric};
use crate::ui::theme::{
    PERFORMANCE_ALERT_GAP as GAP, PERFORMANCE_ALERT_MARGIN as MARGIN,
    PERFORMANCE_ALERT_SIZE as SIZE,
};
use egui::{Color32, Painter, Pos2, Rect, Rgba, Stroke, pos2, vec2};
use std::time::{Duration, Instant};

const COLOR_TRANSITION: Duration = Duration::from_millis(350);
const FADE_OUT: Duration = Duration::from_secs(1);
const ANIMATION_FRAME: Duration = Duration::from_millis(16);
const ORDER: [Metric; 6] = [
    Metric::Rtt,
    Metric::Loss,
    Metric::Jitter,
    Metric::FrameDelay,
    Metric::LocalDelay,
    Metric::PresentationStall,
];

#[derive(Clone, Copy)]
struct IconTransition {
    from: Rgba,
    target: Rgba,
    started_at: Instant,
}

impl IconTransition {
    fn color(self, now: Instant) -> Rgba {
        let progress = now.saturating_duration_since(self.started_at).as_secs_f32()
            / COLOR_TRANSITION.as_secs_f32();
        egui::lerp(self.from..=self.target, smoothstep(progress))
    }

    fn retarget(&mut self, target: Rgba, now: Instant) {
        if self.target != target {
            // Continue from the visible color even if a transition reverses midway.
            self.from = self.color(now);
            self.target = target;
            self.started_at = now;
        }
    }
}

fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn show(ctx: &egui::Context, health: &Assessment, bounds: Rect) {
    let now = Instant::now();
    // Each viewer context owns a bounded set of paint-only animation states.
    let state_id = egui::Id::unique("performance-alert-transitions");
    type Transitions = [Option<IconTransition>; ORDER.len()];
    let count = health.alerts.iter().filter(|a| a.expires_at > now).count();
    if count == 0 || bounds.width() < SIZE + MARGIN * 2.0 || bounds.height() < SIZE + MARGIN * 2.0 {
        ctx.data_mut(|data| data.remove::<Transitions>(state_id));
        return;
    }
    let mut transitions =
        ctx.data_mut(|data| data.get_temp::<Transitions>(state_id).unwrap_or_default());
    let columns =
        (((bounds.width() - MARGIN * 2.0 + GAP) / (SIZE + GAP)) as usize).clamp(1, ORDER.len());
    // Background paint remains below menus and never participates in input routing.
    let painter = ctx
        .layer_painter(egui::LayerId::background())
        .with_clip_rect(bounds);
    let mut index = 0;
    for (metric, transition) in ORDER.into_iter().zip(&mut transitions) {
        let Some(alert) = health
            .alerts
            .iter()
            .find(|a| a.metric == metric && a.expires_at > now)
        else {
            *transition = None;
            continue;
        };
        let row = index / columns;
        let col = index % columns;
        let row_count = (count - row * columns).min(columns);
        let x = bounds.right() - MARGIN - row_count as f32 * (SIZE + GAP)
            + GAP
            + col as f32 * (SIZE + GAP);
        let y = bounds.top() + MARGIN + row as f32 * (SIZE + GAP);
        let remaining = alert.expires_at.saturating_duration_since(now);
        let target = if alert.historical {
            Rgba::from(crate::ui::theme::MUTED) * 0.5
        } else {
            Rgba::from(super::hud::health_color(Some(metric.level(alert.value))))
        };
        let transition = transition.get_or_insert(IconTransition {
            from: Rgba::TRANSPARENT,
            target,
            started_at: now,
        });
        transition.retarget(target, now);
        // Fade to zero at the existing expiry deadline; animation never extends an alert.
        let fade = smoothstep(remaining.as_secs_f32() / FADE_OUT.as_secs_f32());
        let color = Color32::from(transition.color(now) * fade);
        let opacity = color.a() as f32 / 255.0;
        let origin = pos2(x, y);
        paint_icon(
            &painter,
            origin + vec2(0.0, 1.0),
            metric,
            Color32::from_black_alpha((150.0 * opacity) as u8),
            2.8,
        );
        paint_icon(&painter, origin, metric, color, 1.8);
        if now.saturating_duration_since(transition.started_at) < COLOR_TRANSITION
            || remaining <= FADE_OUT
        {
            ctx.request_repaint_after(ANIMATION_FRAME);
        } else {
            ctx.request_repaint_after(remaining.saturating_sub(FADE_OUT));
        }
        ctx.request_repaint_after(remaining);
        index += 1;
    }
    ctx.data_mut(|data| data.insert_temp(state_id, transitions));
}

fn paint_icon(p: &Painter, origin: Pos2, metric: Metric, color: Color32, width: f32) {
    let scale = SIZE / 24.0;
    let stroke = Stroke::new(width * scale, color);
    let point = |x, y| origin + vec2(x, y) * scale;
    let line = |points: &[(f32, f32)]| {
        p.line(points.iter().map(|&(x, y)| point(x, y)).collect(), stroke);
    };
    match metric {
        Metric::Rtt => {
            p.circle_stroke(point(12.0, 14.0), 7.5 * scale, stroke);
            line(&[(9.0, 2.0), (15.0, 2.0)]);
            line(&[(12.0, 2.0), (12.0, 5.0)]);
            line(&[(18.0, 7.0), (20.0, 5.0)]);
            line(&[(12.0, 10.0), (12.0, 14.0)]);
        }
        Metric::Loss => {
            line(&[
                (3.0, 21.0),
                (6.0, 18.0),
                (4.0, 16.0),
                (9.0, 11.0),
                (13.0, 15.0),
                (8.0, 20.0),
                (6.0, 18.0),
            ]);
            line(&[
                (21.0, 3.0),
                (18.0, 6.0),
                (20.0, 8.0),
                (15.0, 13.0),
                (11.0, 9.0),
                (16.0, 4.0),
                (18.0, 6.0),
            ]);
            line(&[(3.0, 3.0), (6.0, 6.0)]);
            line(&[(18.0, 18.0), (21.0, 21.0)]);
        }
        Metric::Jitter => line(&[
            (2.0, 12.0),
            (6.0, 12.0),
            (9.0, 3.0),
            (14.0, 21.0),
            (17.0, 12.0),
            (22.0, 12.0),
        ]),
        Metric::FrameDelay => {
            let points: Vec<_> = (0..=24)
                .map(|i| {
                    let a = std::f32::consts::PI + i as f32 / 24.0 * std::f32::consts::PI;
                    point(12.0 + a.cos() * 9.0, 15.0 + a.sin() * 9.0)
                })
                .collect();
            p.line(points, stroke);
            line(&[(3.0, 15.0), (3.0, 19.0), (21.0, 19.0), (21.0, 15.0)]);
            line(&[(12.0, 15.0), (17.0, 9.0)]);
            p.circle_filled(point(12.0, 15.0), 1.5 * scale, color);
        }
        Metric::LocalDelay => {
            line(&[
                (2.0, 8.0),
                (12.0, 3.0),
                (22.0, 8.0),
                (12.0, 13.0),
                (2.0, 8.0),
            ]);
            line(&[(2.0, 13.0), (12.0, 18.0), (22.0, 13.0)]);
            line(&[(2.0, 17.0), (12.0, 22.0), (22.0, 17.0)]);
        }
        Metric::PresentationStall => {
            p.rect_stroke(
                Rect::from_min_max(point(2.0, 3.0), point(22.0, 17.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            line(&[(12.0, 17.0), (12.0, 21.0)]);
            line(&[(8.0, 21.0), (16.0, 21.0)]);
            line(&[(9.0, 7.0), (9.0, 13.0)]);
            line(&[(15.0, 7.0), (15.0, 13.0)]);
        }
    }
}

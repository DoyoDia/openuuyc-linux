//! Compact pen paths for the whiteboard's remote-visible brand and metadata.
//! Draw RPC accepts strokes rather than text/images, so the header uses simple
//! filled system-font lettering and the application's linked-screen logo.
use super::*;
use crate::ui::theme;

type Path = Vec<[f32; 2]>;

/// The lettering is traced from a GDI rasterization, which has no Linux
/// counterpart yet. The brand mark and its frame still draw; only the text is
/// left out, so the remote side sees a header without a caption.

fn text_paths(text: &str, _size: f32, _weight: i32) -> Result<Vec<Path>> {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        tracing::info!("whiteboard brand lettering is unavailable on this platform");
    });
    let _ = text;
    Ok(Vec::new())
}

fn arc(cx: f32, cy: f32, rx: f32, ry: f32, start: f32, end: f32) -> Path {
    (0..=20)
        .map(|i| {
            let a = (start + (end - start) * i as f32 / 20.).to_radians();
            [cx + a.cos() * rx, cy + a.sin() * ry]
        })
        .collect()
}
fn rounded_logo() -> Vec<(Path, f32, [u8; 3])> {
    let mut outline = vec![[6., 4.], [26., 4.]];
    outline.extend(arc(26., 6., 2., 2., -90., 0.));
    outline.push([28., 26.]);
    outline.extend(arc(26., 26., 2., 2., 0., 90.));
    outline.push([6., 28.]);
    outline.extend(arc(6., 26., 2., 2., 90., 180.));
    outline.push([4., 6.]);
    outline.extend(arc(6., 6., 2., 2., 180., 270.));
    let tile = vec![
        [10., 10.],
        [22., 10.],
        [22., 16.],
        [10., 16.],
        [10., 22.],
        [22., 22.],
    ];
    // Coordinates follow assets/icon-256.png; keep the blue and white interlock.
    let blue = vec![
        [94., 155.],
        [65., 155.],
        [55., 153.],
        [49., 146.],
        [48., 137.],
        [48., 96.],
        [50., 86.],
        [57., 79.],
        [67., 77.],
        [132., 77.],
        [142., 79.],
        [150., 87.],
        [152., 97.],
        [152., 122.],
        [150., 133.],
        [143., 141.],
        [135., 144.],
    ];
    let white = vec![
        [135., 119.],
        [125., 121.],
        [116., 129.],
        [110., 140.],
        [110., 162.],
        [112., 173.],
        [121., 180.],
        [131., 182.],
        [188., 182.],
        [199., 179.],
        [206., 171.],
        [207., 160.],
        [207., 131.],
        [204., 120.],
        [196., 113.],
        [186., 111.],
        [170., 111.],
    ];
    vec![
        (outline, 8., theme::ANNOTATION_BRAND_TILE),
        (tile, 12., theme::ANNOTATION_BRAND_TILE),
        (
            blue.into_iter().map(|[x, y]| [x / 8., y / 8.]).collect(),
            2.5,
            theme::ANNOTATION_BRAND_BLUE,
        ),
        (
            white.into_iter().map(|[x, y]| [x / 8., y / 8.]).collect(),
            2.5,
            theme::ANNOTATION_BRAND_WHITE,
        ),
    ]
}

pub(super) fn build(
    base: u32,
    screen: i32,
    metrics: Metrics,
    background: [u8; 3],
) -> Result<Vec<Stroke>> {
    let lw = metrics.width as f32 * 100. / metrics.dpi as f32;
    let lh = metrics.height as f32 * 100. / metrics.dpi as f32;
    let light = u32::from(background[0]) * 299
        + u32::from(background[1]) * 587
        + u32::from(background[2]) * 114
        > 150_000;
    let colors = if light {
        theme::ANNOTATION_BRAND_ON_LIGHT
    } else {
        theme::ANNOTATION_BRAND_ON_DARK
    };
    let mut paths = Vec::new();
    let padding = theme::ANNOTATION_BRAND_PADDING;
    let logo_scale = theme::ANNOTATION_BRAND_LOGO_SIZE / 32.;
    for (path, width, color) in rounded_logo() {
        paths.push((
            path.into_iter()
                .flat_map(|[x, y]| [[padding + x * logo_scale, padding + y * logo_scale]; 3])
                .collect::<Path>(),
            width * logo_scale,
            color,
        ));
    }
    let info = format!(
        "v{}  ·  {} × {}  ·  {}% DPI",
        env!("CARGO_PKG_VERSION"),
        metrics.width,
        metrics.height,
        metrics.dpi
    );
    let x = padding + theme::ANNOTATION_BRAND_LOGO_SIZE + theme::ANNOTATION_BRAND_GAP;
    for (text, y, size, weight, color) in [
        (
            "OpenUUYC",
            padding,
            theme::ANNOTATION_BRAND_TITLE_SIZE,
            600,
            colors[0],
        ),
        (
            info.as_str(),
            padding + theme::ANNOTATION_BRAND_TITLE_SIZE + theme::ANNOTATION_BRAND_GAP * 0.5,
            theme::ANNOTATION_BRAND_INFO_SIZE,
            400,
            colors[1],
        ),
    ] {
        for path in text_paths(text, size, weight)? {
            paths.push((
                path.into_iter()
                    .map(|[px, py]| [x + px, y + py])
                    .collect::<Path>(),
                0.75,
                color,
            ));
        }
    }
    let extent =
        paths
            .iter()
            .flat_map(|(path, _, _)| path.iter())
            .fold([1_f32, 1_f32], |mut size, p| {
                size[0] = size[0].max(p[0] + padding);
                size[1] = size[1].max(p[1] + padding);
                size
            });
    let scale = (lw / extent[0]).min(lh / extent[1]).min(1.);
    if paths.len() >= BOARD_SLOT_IDS as usize {
        bail!("白板标识内容超出范围");
    }
    Ok(paths
        .into_iter()
        .enumerate()
        .map(|(i, (points, width, rgb))| Stroke {
            id: base + 1 + i as u32,
            screen,
            points: points
                .into_iter()
                .map(|[x, y]| Point {
                    x: (x * scale / lw).clamp(0., 1.),
                    y: (y * scale / lh).clamp(0., 1.),
                })
                .collect(),
            style: Style {
                argb: u32::from_be_bytes([255, rgb[0], rgb[1], rgb[2]]),
                width: width * scale,
            },
        })
        .collect())
}

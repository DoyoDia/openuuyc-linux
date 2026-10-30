//! Decoder diagnostic report types; interactive probing is not implemented on Linux.
use crate::media::{LocalDisplayInfo, VideoCodec};

use anyhow::Result;
use std::sync::{Arc, atomic::AtomicBool, mpsc::Sender};

#[derive(Clone, Copy)]
pub(crate) struct Format {
    codec: VideoCodec,
    chroma: u8,
    depth: u8,
}
impl Format {
    fn label(self) -> String {
        format!(
            "{} · {} · {}位",
            if self.codec == VideoCodec::Av1 {
                "AV1"
            } else if self.codec == VideoCodec::H264 {
                "H.264"
            } else {
                "H.265"
            },
            if self.chroma == 1 { "420" } else { "444" },
            self.depth
        )
    }
    fn sample(self, size: (u32, u32)) -> Option<&'static [u8]> {
        macro_rules! sample {
            ($name:literal, $extension:literal) => {
                match size {
                    (1280, 720) => Some(
                        include_bytes!(concat!("fixtures/matrix/", $name, "_1280x720", $extension))
                            .as_slice(),
                    ),
                    (1920, 1080) => Some(
                        include_bytes!(concat!(
                            "fixtures/matrix/",
                            $name,
                            "_1920x1080",
                            $extension
                        ))
                        .as_slice(),
                    ),
                    (2560, 1440) => Some(
                        include_bytes!(concat!(
                            "fixtures/matrix/",
                            $name,
                            "_2560x1440",
                            $extension
                        ))
                        .as_slice(),
                    ),
                    (3840, 2160) => Some(
                        include_bytes!(concat!(
                            "fixtures/matrix/",
                            $name,
                            "_3840x2160",
                            $extension
                        ))
                        .as_slice(),
                    ),
                    _ => None,
                }
            };
        }
        match (self.codec, self.chroma, self.depth) {
            (VideoCodec::Av1, 1, 8) => sample!("av1_8", ".obu"),
            (VideoCodec::Av1, 1, 10) => sample!("av1_10", ".obu"),
            (VideoCodec::Av1, 3, 8) => sample!("av1_444_8", ".obu"),
            (VideoCodec::Av1, 3, 10) => sample!("av1_444_10", ".obu"),

            (VideoCodec::H264, 1, 8) => sample!("h264", ".annexb"),
            (VideoCodec::H264, 3, 8) => sample!("h264_444", ".annexb"),
            (VideoCodec::H265, 1, 8) => sample!("hevc", ".annexb"),
            (VideoCodec::H265, 1, 10) => sample!("main10", ".annexb"),
            (VideoCodec::H265, 3, 8) => sample!("hevc_444", ".annexb"),
            (VideoCodec::H265, 3, 10) => sample!("hevc_444_10", ".annexb"),
            _ => None,
        }
    }
}
fn formats() -> Vec<Format> {
    [VideoCodec::H264, VideoCodec::H265, VideoCodec::Av1]
        .into_iter()
        .flat_map(|codec| {
            [1, 3].into_iter().flat_map(move |chroma| {
                [8, 10].map(|depth| Format {
                    codec,
                    chroma,
                    depth,
                })
            })
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Pending,
    Passed,
    Failed,
    Unsupported,
    Busy,
    Unchecked,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "待检查",
            Self::Passed => "通过",
            Self::Failed => "失败",
            Self::Unsupported => "未实现",
            Self::Busy => "资源占用",
            Self::Unchecked => "未检查",
        }
    }
}
#[derive(Clone)]
pub(crate) struct Cell {
    pub status: Status,
    pub detail: String,
}
pub(crate) struct Row {
    pub format: String,
    pub cells: Vec<Cell>,
}
pub(crate) struct Backend {
    pub name: String,
    pub device: Option<String>,
    pub sizes: Vec<(u32, u32)>,
    pub rows: Vec<Row>,
}
#[derive(Default)]
pub(crate) struct Report {
    pub backends: Vec<Backend>,
    pub message: String,
}
pub(crate) enum Event {
    Backend(Backend),
    Cell(usize, usize, usize, Cell),
    Finished(String),
}

/// The check drives DXVA11 adapters one by one; VA-API has no counterpart yet,
/// so there is nothing to report here rather than a partial matrix.

pub(crate) fn run(
    _display: LocalDisplayInfo,
    _tx: &Sender<Event>,
    _cancel: Arc<AtomicBool>,
) -> Result<()> {
    anyhow::bail!("Linux 暂未实现交互式解码检查")
}

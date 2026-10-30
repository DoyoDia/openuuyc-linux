//! Desktop GUI ownership and shared application controls.
use anyhow::Result;

pub(crate) mod chrome;
pub(crate) mod gfx;

#[path = "tray_linux.rs"]
mod tray;

/// 已有实例的唤起通道，通过 Unix socket 传递请求。

#[path = "control_linux.rs"]
pub(crate) mod control;

/// SIGTERM/SIGINT 转成一次正常退出，服务被 `systemctl stop` 时才有收尾。

#[path = "shutdown_linux.rs"]
pub(crate) mod shutdown;

pub(crate) mod window_manager;
mod windows;

mod app;
pub(crate) mod branding;
pub(crate) mod controls;
pub(crate) mod fonts;
pub(crate) mod theme;
use app::AppFactory;

use app::AppSession;
pub(crate) use app::{App, WindowConfig};

pub(crate) fn run(config: WindowConfig, factory: AppFactory) -> Result<()> {
    windows::run(config, factory)
}

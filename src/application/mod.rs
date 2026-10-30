//! Application ownership and module boundaries.

pub mod app;
pub mod bootstrap;
pub mod viewer;
pub(crate) mod viewer_shortcuts;
pub(crate) mod wallpaper;
/// 列出或接上本机正在运行的被控终端会话（命令行入口）。
///
/// 会话跑在 GUI 进程里，这里只是经 Unix socket 连过去；关闭这个终端不会
/// 结束会话。
pub fn terminal_attach(session: Option<&str>, list: bool, new: bool) -> anyhow::Result<()> {
    crate::features::host::terminal::local::run_client(session, list, new)
}

/// Internal crash-recovery role of the same executable.
pub fn display_recovery(token: &str) -> anyhow::Result<()> {
    crate::features::host::displays::recovery::watch(token)
}

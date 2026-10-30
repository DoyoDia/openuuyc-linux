//! One control center per desktop login session, independent of the EXE name.

/// 第二个进程要不要把已经在跑的控制中心叫到前台，以及顺带交给它的东西。
#[derive(Clone, Copy)]
pub enum Activation<'a> {
    /// 再点一次图标：把窗口叫出来就行。
    Show,
    /// 系统按 `uuremote://` 协议注册拉起的协助链接。链接原样转发，由接手的
    /// 那个进程解析——窗口消息只负责搬运。
    Link(&'a str),
    /// 后台启动，不打扰已经在跑的实例。
    Quiet,
}

mod platform {
    use super::Activation;
    use anyhow::{Context, Result};
    use std::fs::{File, OpenOptions};
    use std::path::PathBuf;

    /// The advisory lock lives as long as this handle; the file itself stays behind.
    pub struct Instance(#[allow(dead_code)] File);

    fn lock_path() -> PathBuf {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|path| path.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        // Per-user, so a shared /tmp fallback cannot collide across accounts.
        base.join(format!("openuuyc-control-center-{}.lock", unsafe {
            libc::getuid()
        }))
    }

    /// 抢实例锁。抢不到就把这次启动的来意交给已经在跑的那个实例，然后自己去退出。
    pub fn acquire(activation: Activation<'_>) -> Result<Option<Instance>> {
        let path = lock_path();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("创建程序实例保护失败：{}", path.display()))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Instance(file))),
            Err(std::fs::TryLockError::WouldBlock) => match activation {
                // 运行的实例可能是服务，也可能只是收在托盘里，先把它的窗口叫出来；
                // 叫不动（旧版本、通道不可用）就保持沉默，别假装又起了一份。
                Activation::Show if crate::ui::control::activate().is_ok() => Ok(None),
                Activation::Show | Activation::Quiet => {
                    eprintln!("OpenUUYC 已在运行，请勿重复启动。");
                    Ok(None)
                }
                // 链接是用户点出来的，交不出去就得说出来，不能悄悄丢掉。
                Activation::Link(uri) => {
                    crate::ui::control::deliver_link(uri)?;
                    Ok(None)
                }
            },
            Err(std::fs::TryLockError::Error(error)) => {
                eprintln!("无法启动 OpenUUYC：{error}");
                Err(anyhow::Error::new(error).context("锁定程序实例保护失败"))
            }
        }
    }

    /// X11 and Wayland have no portable window-property handshake; the lock file
    /// is the whole reservation and the running window is left untouched.
    pub(crate) fn register_window(_window: &winit::window::Window) -> Result<()> {
        Ok(())
    }

    /// Only the Windows updater asks a running control center to exit.
    pub(crate) fn take_update_exit() -> bool {
        false
    }
}

pub use platform::{Instance, acquire};

pub(crate) use platform::{register_window, take_update_exit};

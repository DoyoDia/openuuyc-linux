//! Session-owned drawing on the interactive default desktop.
//!
//! Linux has no overlay renderer yet: opening a board is refused with a
//! reason, everything else answers as an idle Windows host would.

use super::{Action, Context};

use anyhow::Result;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(super) struct Backend;

impl Backend {
    pub fn interval(&self) -> Duration {
        Duration::from_millis(50)
    }
    pub fn exchange(
        &mut self,
        context: Context,
        command: Option<Action>,
        cancel: &CancellationToken,
    ) -> Result<i32> {
        if !context.allowed || cancel.is_cancelled() {
            return Ok(4);
        }
        match command.as_ref().map(Action::toggle) {
            None | Some(Some(false)) => Ok(0),
            Some(Some(true)) => anyhow::bail!("Linux 暂不支持被控端批注（远程画笔）"),
            Some(None) => Ok(4),
        }
    }
}

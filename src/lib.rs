//! OpenUUYC: UU protocol interoperability and Linux application composition.

#[cfg(not(target_os = "linux"))]
compile_error!("OpenUUYC Linux only supports Linux targets");

pub const APP_NAME: &str = "OpenUUYC";
pub(crate) const VIEWER_TITLE_PREFIX: &str = "OpenUUYC — ";

pub mod account;
pub mod application;
pub mod diagnostics;
pub mod features;
mod lifecycle;
pub mod media;
pub mod protocol;
pub mod session;
pub mod transport;

pub mod plugins;
mod ui;

mod platform;

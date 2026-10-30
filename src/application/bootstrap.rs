//! Linux process environment setup before opening the GUI.

pub fn initialize() -> anyhow::Result<()> {
    crate::platform::linux::gamescope::initialize_environment();
    Ok(())
}

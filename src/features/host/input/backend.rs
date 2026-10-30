//! Local and service execution share the same engine and ownership contract.

use super::{engine::Engine, geometry::Geometry, wire::Event};

use anyhow::Result;

pub(super) struct Native {
    engine: Engine,
    // Present only in the already verified service-owned resident. The GUI
    // cannot select this authority by filename, configuration or elevation.
}

impl Native {
    /// Whether the session this engine injects into is still the active one.
    /// Linux runs only inside the signed-in desktop, which is always current.
    fn current(&self) -> bool {
        true
    }
    /// Whether this engine is the installed service's own resident one.
    fn service(&self) -> bool {
        false
    }
    fn check(&self) -> Result<()> {
        anyhow::ensure!(self.current(), "输入所属Windows会话已结束");
        Ok(())
    }
}
pub(super) enum Backend {
    Local(Native),
}
impl Backend {
    pub fn new(
        policy: super::wire::Policy,
        require_service: bool,
        permitted: impl Fn() -> bool,
    ) -> Result<Self> {
        let _ = permitted;
        anyhow::ensure!(!require_service, "系统被控服务尚未恢复");
        Ok(Self::Local(Native {
            engine: Engine::new(false, policy, std::process::id(), false)?,
        }))
    }
    pub fn service(&self) -> bool {
        match self {
            Self::Local(native) => native.service(),
        }
    }
    pub fn healthy(&self) -> bool {
        match self {
            Self::Local(native) => native.current(),
        }
    }
    pub fn backend(&self) -> &str {
        match self {
            Self::Local(e) => e.engine.backend(),
        }
    }
    pub fn apply(
        &mut self,
        events: Vec<Event>,
        geometry: Geometry,
        permitted: impl Fn() -> bool,
    ) -> Result<()> {
        match self {
            Self::Local(e) => {
                e.check()?;

                let allowed = &permitted;
                for event in events {
                    e.engine.apply(event, geometry.clone(), &allowed)?;
                    if e.engine.take_sas() {}
                }
                Ok(())
            }
        }
    }
    /// 请合成器重新判定一次指针底下是谁——Wayland 的拖放接取面要靠这一下。只有
    /// Linux 有这条：别处的合成器自己会在面上屏时重判。

    pub fn realign_pointer(&mut self) -> Result<bool> {
        match self {
            Self::Local(native) => native.engine.realign_pointer(),
        }
    }
    pub fn release(&mut self) -> Result<()> {
        match self {
            Self::Local(e) => e.engine.release(),
        }
    }
    pub fn synchronize(&mut self, geometry: Geometry) -> Result<()> {
        match self {
            Self::Local(e) => {
                e.check()?;
                e.engine.synchronize(geometry)
            }
        }
    }
    pub fn configure(&mut self, configuration: super::config::Configuration) -> Result<()> {
        match self {
            Self::Local(e) => {
                e.check()?;
                e.engine.configure(configuration)
            }
        }
    }
    pub fn mouse_policy(&self) -> super::config::MousePolicy {
        match self {
            Self::Local(e) => e.engine.mouse_policy(),
        }
    }
    pub fn tick(&mut self) -> Result<()> {
        match self {
            Self::Local(e) => {
                e.check()?;
                e.engine.tick()
            }
        }
    }
}

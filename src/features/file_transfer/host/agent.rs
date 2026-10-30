//! Same protocol engine in portable mode and in an ordinary-user service child.
//!
//! Linux has no system service yet, so the engine always runs in the desktop
//! user's own process there (the portable mode).
use super::*;

use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(prost::Message)]
struct Request {
    #[prost(message, optional, tag = "6")]
    capabilities: Option<Capabilities>,
    #[prost(uint64, tag = "5")]
    revision: u64,
    #[prost(string, tag = "1")]
    scope: String,
    #[prost(bool, tag = "2")]
    enabled: bool,
    #[prost(bytes, repeated, tag = "3")]
    packets: Vec<Vec<u8>>,
    #[prost(bool, tag = "4")]
    close: bool,
}
#[derive(prost::Message)]
struct Reply {
    #[prost(message, repeated, tag = "2")]
    notices: Vec<notices::Notice>,
    #[prost(message, repeated, tag = "1")]
    packets: Vec<Packet>,
}
pub(crate) struct Backend {
    pub notices: notices::Journal,
    pub input: mpsc::Sender<(u64, Vec<u8>)>,
    policy: Arc<Mutex<(u64, bool, Capabilities)>>,
    pub ready: Arc<AtomicBool>,
    pub error: Arc<Mutex<Option<String>>>,
    pub output: mpsc::Receiver<(u64, Packet)>,
    worker: tokio::task::JoinHandle<()>,
    stop: CancellationToken,
}
impl Backend {
    pub fn new(scope: String, stop: CancellationToken) -> Self {
        let (input, rx) = mpsc::channel(32);
        let (tx, output) = mpsc::channel(32);
        let policy = Arc::new(Mutex::new((0, false, Capabilities::default())));
        let settings = policy.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let available = ready.clone();
        let error = Arc::new(Mutex::new(None));
        let status = error.clone();
        let cancel = stop.clone();
        let notices = notices::Journal::default();
        let updates = notices.clone();
        let activity = crate::platform::host_service::activity::Work::new();
        let worker = tokio::task::spawn_blocking(move || {
            let _activity = activity;
            if let Err(e) = worker(scope, rx, tx, settings, available, cancel, updates) {
                tracing::warn!(error=%format!("{e:#}"), "host file executor stopped");
                *super::super::lock(&status) = Some(e.to_string());
            }
        });
        Self {
            notices,
            input,
            policy,
            ready,
            error,
            output,
            worker,
            stop,
        }
    }
    pub fn policy(&self, enabled: bool, capabilities: Capabilities) -> u64 {
        let mut state = super::super::lock(&self.policy);
        if state.1 != enabled {
            state.0 = state.0.wrapping_add(1);
            state.1 = enabled;
        }
        state.2 = capabilities;
        state.0
    }
    pub async fn close(self) -> Vec<notices::Notice> {
        self.stop.cancel();
        let _ = self.worker.await;
        self.notices.take()
    }
}
struct Local {
    journal: notices::Journal,
    scope: String,
    revision: u64,
    engine: Option<Engine>,
    tx: mpsc::Sender<Packet>,
    rx: mpsc::Receiver<Packet>,
    runtime: tokio::runtime::Runtime,
}
impl Local {
    fn new(scope: String) -> Result<Self> {
        let (tx, rx) = mpsc::channel(32);
        Ok(Self {
            journal: Default::default(),
            scope,
            revision: 0,
            engine: None,
            tx,
            rx,
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .thread_name("file-transfer-user")
                .build()?,
        })
    }
    fn exchange(&mut self, request: Request) -> Result<Reply> {
        ensure!(
            request.scope == self.scope && request.packets.len() <= 4,
            "文件代理请求无效"
        );
        if (!request.enabled || request.close || request.revision != self.revision)
            && self.engine.is_some()
        {
            let engine = self.engine.take().unwrap();
            let denied = if request.close {
                Vec::new()
            } else {
                engine.revoked()
            };
            self.runtime.block_on(engine.close());
            while self.rx.try_recv().is_ok() {}
            for packet in denied {
                self.tx.try_send(packet)?;
            }
        }
        self.revision = request.revision;
        if request.enabled && !request.close && self.engine.is_none() {
            self.engine = Some(Engine::new(
                &self.scope,
                self.tx.clone(),
                self.journal.clone(),
            )?);
        }
        let mut packets = Vec::new();
        for packet in request.packets {
            ensure!(packet.len() < WIRE, "文件代理消息过大");
            let rejected = if let Some(engine) = &mut self.engine {
                match self
                    .runtime
                    .block_on(engine.receive(&packet, request.capabilities.unwrap_or_default()))
                {
                    Ok(()) => None,
                    Err(error) => {
                        tracing::warn!(%error, "host file request rejected");
                        Some(failure(&error))
                    }
                }
            } else {
                Some(4)
            };
            if let Some(code) = rejected {
                // A business rejection belongs to this request, not the user
                // executor. Replies have bounded space independent of a busy
                // transfer's output queue; unrelated tasks continue running.
                match reject(&packet, code) {
                    Ok(Some(reply)) => packets.push(reply),
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(%error, "host file rejection has no reply envelope")
                    }
                }
            }
        }
        for _ in packets.len()..4 {
            match self.rx.try_recv() {
                Ok(p) => packets.push(p),
                Err(_) => break,
            }
        }
        Ok(Reply {
            packets,
            notices: self.journal.take(),
        })
    }
}
impl Drop for Local {
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take() {
            self.runtime.block_on(engine.close());
        }
    }
}

/// The SYSTEM service process, which must hand disk work to a user child.

enum Remote {}

impl Remote {
    fn new(_stop: &CancellationToken) -> Result<Self> {
        anyhow::bail!("Linux 没有系统服务进程")
    }
    fn exchange(&self, _request: &Request, _stop: &CancellationToken) -> Result<Reply> {
        match *self {}
    }
}
fn worker(
    scope: String,
    mut input: mpsc::Receiver<(u64, Vec<u8>)>,
    output: mpsc::Sender<(u64, Packet)>,
    policy: Arc<Mutex<(u64, bool, Capabilities)>>,
    ready: Arc<AtomicBool>,
    stop: CancellationToken,
    updates: notices::Journal,
) -> Result<()> {
    let system = false;
    let mut local = if system {
        None
    } else {
        Some(Local::new(scope.clone())?)
    };
    let mut remote: Option<Remote> = None;
    let mut retry = Instant::now();
    let result = (|| -> Result<()> {
        while !stop.is_cancelled() {
            let mut packets = Vec::new();
            match input.try_recv() {
                Ok(p) => packets.push(p),
                Err(mpsc::error::TryRecvError::Disconnected) => break,
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            for _ in 0..3 {
                match input.try_recv() {
                    Ok(p) => packets.push(p),
                    Err(_) => break,
                }
            }
            let (revision, requested, capabilities) = *super::super::lock(&policy);
            let packets = packets
                .into_iter()
                .filter(|(g, _)| *g == revision)
                .map(|(_, p)| p)
                .collect();

            if system && remote.is_none() && requested && Instant::now() >= retry {
                match Remote::new(&stop) {
                    Ok(r) => remote = Some(r),
                    Err(_) => retry = Instant::now() + Duration::from_secs(1),
                }
            }
            ready.store(local.is_some() || remote.is_some(), Ordering::Release);
            let request = Request {
                capabilities: Some(capabilities),
                revision,
                scope: scope.clone(),
                enabled: requested,
                packets,
                close: false,
            };
            let reply = if let Some(local) = &mut local {
                local.exchange(request)?
            } else if let Some(remote) = &remote {
                remote.exchange(&request, &stop)?
            } else {
                Reply {
                    notices: Vec::new(),
                    packets: request
                        .packets
                        .iter()
                        .filter_map(|p| reject(p, if requested { 8 } else { 4 }).transpose())
                        .collect::<Result<Vec<_>>>()?,
                }
            };
            for notice in reply.notices {
                updates.publish(notice);
            }
            for packet in reply.packets {
                let mut packet = (revision, packet);
                loop {
                    if stop.is_cancelled() {
                        return Ok(());
                    }
                    match output.try_send(packet) {
                        Ok(()) => break,
                        Err(mpsc::error::TrySendError::Full(p)) => {
                            packet = p;
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        Err(_) => return Ok(()),
                    }
                }
            }
        }
        Ok(())
    })();
    if let Some(remote) = remote {
        let close = Request {
            capabilities: None,
            revision: 0,
            scope,
            enabled: false,
            packets: Vec::new(),
            close: true,
        };
        // A cancelled exchange may have written only a frame prefix, or read
        // only part of a reply. Never append another request to that stream.
        // It would become the missing payload of the previous frame and leave
        // the user backend waiting for its ten-second receive timeout.
        if result.is_ok() {
            if let Ok(reply) = remote.exchange(&close, &CancellationToken::new()) {
                for notice in reply.notices {
                    updates.publish(notice);
                }
            }
        }
    }
    if let Some(local) = &mut local {
        if let Some(engine) = local.engine.take() {
            local.runtime.block_on(engine.close());
        }
        for notice in local.journal.take() {
            updates.publish(notice);
        }
    }
    drop(local);
    if stop.is_cancelled() { Ok(()) } else { result }
}

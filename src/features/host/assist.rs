//! Host assistance policy and challenge handling, owned by the online account.

use crate::session::host_client::HostClient;
use anyhow::{Context, Result, ensure};
use rand::RngExt;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Mode {
    #[default]
    Password,
    Confirmation,
    PasswordConfirmation,
}
impl Mode {
    pub const ALL: [Self; 3] = [
        Self::Password,
        Self::Confirmation,
        Self::PasswordConfirmation,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Password => "密码验证",
            Self::Confirmation => "本机确认",
            Self::PasswordConfirmation => "密码验证并经本机确认",
        }
    }
    pub fn wire(self) -> &'static str {
        match self {
            Self::Password => "by_password",
            Self::Confirmation => "by_confirmation",
            Self::PasswordConfirmation => "password_confirmation",
        }
    }
    pub fn needs_password(self) -> bool {
        self != Self::Confirmation
    }
    pub fn needs_confirmation(self) -> bool {
        self != Self::Password
    }
}

// Deliberately no Debug: this configuration contains the local access code.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Passwords {
    #[default]
    Random,
    Custom,
    Both,
}
impl Passwords {
    pub const ALL: [Self; 3] = [Self::Random, Self::Custom, Self::Both];
    pub fn label(self) -> &'static str {
        match self {
            Self::Random => "随机验证码",
            Self::Custom => "自定义密码",
            Self::Both => "两者均可",
        }
    }
    pub fn random(self) -> bool {
        self != Self::Custom
    }
    pub fn custom(self) -> bool {
        self != Self::Random
    }
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Settings {
    pub enabled: bool,
    pub mode: Mode,
    pub code: String,
    #[serde(default)]
    pub passwords: Passwords,
    #[serde(default)]
    pub custom_code: String,
}
impl Settings {
    pub fn valid_custom_code(code: &str) -> bool {
        (8..=16).contains(&code.len())
            && code.bytes().all(|b| b.is_ascii_alphanumeric())
            && code.bytes().any(|b| b.is_ascii_alphabetic())
            && code.bytes().any(|b| b.is_ascii_digit())
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.code.is_empty()
                || ((6..=32).contains(&self.code.len())
                    && self.code.bytes().all(|b| b.is_ascii_alphanumeric())),
            "验证码须为6至32位字母或数字"
        );
        ensure!(
            self.custom_code.is_empty() || Self::valid_custom_code(&self.custom_code),
            "自定义密码须为8至16位字母和数字，且同时包含两者"
        );
        if self.enabled && self.mode.needs_password() {
            ensure!(
                !self.passwords.random() || !self.code.is_empty(),
                "请先生成验证码"
            );
            ensure!(
                !self.passwords.custom() || !self.custom_code.is_empty(),
                "请先设置自定义密码"
            );
        }
        Ok(())
    }
    fn verifiers(&self, salt: &str) -> (String, Option<String>) {
        match self.passwords {
            Passwords::Random => (verifier(salt, &self.code), None),
            Passwords::Custom => (verifier(salt, &self.custom_code), None),
            Passwords::Both => (
                verifier(salt, &self.code),
                Some(verifier(salt, &self.custom_code)),
            ),
        }
    }
    pub fn refresh_code(&mut self) {
        const LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
        const DIGITS: &[u8] = b"23456789";
        const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::rng();
        let mut code = [0u8; 8];
        code[0] = LETTERS[rng.random_range(0..LETTERS.len())];
        code[1] = DIGITS[rng.random_range(0..DIGITS.len())];
        for byte in &mut code[2..] {
            *byte = ALPHABET[rng.random_range(0..ALPHABET.len())];
        }
        code.shuffle(&mut rng);
        self.code = code.into_iter().map(char::from).collect();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Confirmation {
    pub id: String,
    pub token: String,
    pub name: String,
    pub expires_at: i64,
    pub responding: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Attempt {
    pub id: String,
    pub verifying: bool,
    pub expires_at: i64,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub settings: Settings,
    pub connect_id: String,
    pub ready: bool,
    pub error: Option<String>,
    pub pending: Option<Confirmation>,
    pub attempt: Option<Attempt>,
}

enum Push {
    Mode(String),
    Verify {
        id: String,
        salt: String,
    },
    Confirm {
        id: String,
        name: String,
    },
    Cancel(String),
    Answer {
        id: String,
        token: String,
        allow: bool,
    },
    Refresh,
    Reject(String),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Action {
    Refresh,
    Answer {
        id: String,
        token: String,
        allow: bool,
    },
}

struct State {
    snapshot: Snapshot,
    requests: Option<mpsc::Sender<Queued>>,
    revision: u64,
    operations: CancellationToken,
    admission: CancellationToken,
    pending_deadline: Option<Instant>,
    pending_room: Option<CancellationToken>,
    attempt_deadline: Option<Instant>,
    ui_seen: Option<Instant>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            snapshot: Snapshot::default(),
            requests: None,
            revision: 0,
            operations: CancellationToken::new(),
            admission: CancellationToken::new(),
            pending_deadline: None,
            pending_room: None,
            attempt_deadline: None,
            ui_seen: None,
        }
    }
}
struct Queued {
    event: Push,
    revision: u64,
    room: Option<CancellationToken>,
}
impl Queued {
    fn current(&self, revision: u64) -> bool {
        self.revision == revision && self.room.as_ref().is_none_or(|room| !room.is_cancelled())
    }
}
impl State {
    fn enqueue(&self, event: Push, room: Option<CancellationToken>) -> Result<()> {
        ensure!(
            room.as_ref().is_none_or(|room| !room.is_cancelled()),
            "协助请求已结束"
        );
        self.requests
            .as_ref()
            .context("协助后台未就绪")?
            .try_send(Queued {
                event,
                revision: self.revision,
                room,
            })
            .map_err(|_| anyhow::anyhow!("协助后台忙，请稍后重试"))
    }
}
#[derive(Clone, Default)]
pub(crate) struct Handle(Arc<Mutex<State>>);
impl Handle {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn settings(&self) -> Settings {
        self.lock().snapshot.settings.clone()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.lock().snapshot.clone()
    }
    pub fn replace(&self, snapshot: Snapshot) {
        self.lock().snapshot = snapshot;
    }
    pub fn configure(&self, settings: Settings) -> Result<bool> {
        settings.validate()?;
        let mut state = self.lock();
        if settings == state.snapshot.settings {
            return Ok(false);
        }
        state.operations.cancel();
        state.operations = CancellationToken::new();
        if !settings.enabled || settings.mode != state.snapshot.settings.mode {
            state.admission.cancel();
            state.admission = CancellationToken::new();
        }
        state.revision = state.revision.wrapping_add(1);
        state.snapshot.ready &= settings.enabled;
        state.snapshot.settings = settings;
        let room = state.pending_room.take();
        if let Some(pending) = state.snapshot.pending.take() {
            let _ = state.enqueue(Push::Reject(pending.id), room);
        }
        state.pending_deadline = None;
        state.snapshot.attempt = None;
        state.attempt_deadline = None;
        state.snapshot.error = None;
        Ok(true)
    }
    pub fn unavailable(&self) {
        let mut state = self.lock();
        state.revision = state.revision.wrapping_add(1);
        state.operations.cancel();
        state.operations = CancellationToken::new();
        state.admission.cancel();
        state.admission = CancellationToken::new();
        state.snapshot.ready = false;
        state.snapshot.pending = None;
        state.pending_deadline = None;
        state.pending_room = None;
        state.snapshot.attempt = None;
        state.attempt_deadline = None;
    }
    pub fn admission(&self) -> Result<CancellationToken> {
        let state = self.lock();
        ensure!(
            state.snapshot.settings.enabled && state.snapshot.ready,
            "本机未允许远程协助"
        );
        Ok(state.admission.clone())
    }
    pub fn touch_ui(&self) {
        self.lock().ui_seen = Some(Instant::now());
    }
    pub fn ui_present(&self) -> bool {
        self.lock()
            .ui_seen
            .is_some_and(|at| at.elapsed() < Duration::from_secs(3))
    }
    pub fn act(&self, action: Action) -> Result<()> {
        match action {
            Action::Refresh => self.refresh(),
            Action::Answer { id, token, allow } => self.answer(id, token, allow),
        }
    }
    pub fn answer(&self, id: String, token: String, allow: bool) -> Result<()> {
        let state = self.lock();
        ensure!(
            state
                .snapshot
                .pending
                .as_ref()
                .is_some_and(|p| p.id == id && p.token == token && !p.responding),
            "协助请求已结束"
        );
        state.enqueue(
            Push::Answer { id, token, allow },
            state.pending_room.clone(),
        )
    }
    pub fn refresh(&self) -> Result<()> {
        self.lock().enqueue(Push::Refresh, None)
    }
    pub fn push(&self, value: &serde_json::Value, room: &CancellationToken) -> bool {
        let Some(kind) = value.get("type").and_then(|v| v.as_str()) else {
            return false;
        };
        if !matches!(
            kind,
            "get_control_mode"
                | "remote_control"
                | "remote_control_by_confirmation"
                | "remote_control_by_confirmation_cancel"
        ) {
            return false;
        }
        let data = &value["data"];
        let field = |name: &str| {
            data.get(name)
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty() && v.len() <= 256 && !v.chars().any(char::is_control))
        };
        let Some(id) = field("control_id").map(str::to_owned) else {
            return true;
        };
        let event = match kind {
            "get_control_mode" => Push::Mode(id),
            "remote_control" => {
                let Some(salt) = field("salt") else {
                    return true;
                };
                Push::Verify {
                    id,
                    salt: salt.into(),
                }
            }
            "remote_control_by_confirmation" => Push::Confirm {
                id,
                name: field("name").unwrap_or("远端设备").into(),
            },
            _ => Push::Cancel(id),
        };
        let _ = self.lock().enqueue(event, Some(room.clone()));
        true
    }
}

pub(crate) struct Running {
    cancel: CancellationToken,
    task: JoinHandle<()>,
    handle: Handle,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.task.abort();
        self.handle.unavailable();
        self.handle.lock().requests = None;
    }
}
impl Running {
    pub fn start(client: HostClient) -> Self {
        let handle = client.host.assistance.clone();
        let (tx, rx) = mpsc::channel(32);
        handle.lock().requests = Some(tx);
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        let task = tokio::spawn(async move {
            run(client, rx, stop).await;
        });
        Self {
            cancel,
            task,
            handle,
        }
    }
}

fn verifier(salt: &str, code: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(salt.as_bytes());
    hash.update(code.as_bytes());
    hex::encode(hash.finalize())
}

enum Completion {
    Identity {
        revision: u64,
        result: Option<Result<String>>,
    },
    Cancelled,
    Reply {
        revision: u64,
        room: CancellationToken,
        request: Option<String>,
        result: Result<()>,
    },
}

fn report_failure(state: &mut State, error: &anyhow::Error) {
    let code = error
        .downcast_ref::<crate::account::api::ApiFailure>()
        .map(|e| e.code);
    tracing::warn!(code, "host assistance request failed");
    state.snapshot.error = Some(match code {
        Some(code) => format!("协助请求失败（{code}），请重试"),
        None => "协助请求未完成，请检查网络后重试".into(),
    });
}

async fn run(client: HostClient, mut rx: mpsc::Receiver<Queued>, stop: CancellationToken) {
    let handle = client.host.assistance.clone();
    let mut jobs = JoinSet::new();
    let mut replies = Vec::<(String, &'static str, AbortHandle)>::new();
    let mut identity_attempted = false;
    let mut identity_pending = false;
    let mut requested_revision = 0;
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut answered = std::collections::VecDeque::<(String, Instant)>::new();
    let ended = client.ended();
    loop {
        replies.retain(|(_, _, task)| !task.is_finished());
        let event = tokio::select! {
            biased;
            _=stop.cancelled()=>break,
            _=ended.cancelled()=>break,
            Some(result)=jobs.join_next(), if !jobs.is_empty()=>{
                match result {
                    Ok(Completion::Identity { revision, result }) => {
                        identity_pending = false;
                        let mut state = handle.lock();
                        if revision == state.revision {
                            match result {
                                Some(Ok(id)) => state.snapshot.connect_id = id,
                                Some(Err(error)) => report_failure(&mut state, &error),
                                None => (),
                            }
                        }
                    }
                    Ok(Completion::Cancelled) => (),
                    Ok(Completion::Reply { revision, room, request, result }) => {
                        let mut state = handle.lock();
                        if revision == state.revision && !room.is_cancelled() {
                            if let Some(token) = request {
                                if state.snapshot.pending.as_ref().is_some_and(|p|p.token == token) {
                                    state.snapshot.pending = None;
                                    state.pending_deadline = None;
                                    state.pending_room = None;
                                    state.snapshot.error = None;
                                }
                            }
                            match result {
                                Ok(()) => state.snapshot.error = None,
                                Err(error) => report_failure(&mut state, &error),
                            }
                        }
                    }
                    Err(error) if error.is_cancelled() => (),
                    Err(_) => {
                        handle.unavailable();
                        handle.lock().snapshot.error = Some("协助后台已停止，请重新打开程序".into());
                        break;
                    }
                }
                continue;
            }
            value=rx.recv()=>match value {Some(v)=>Some(v),None=>break},
            _=tick.tick()=>None,
        };
        let allowed = client.is_active() && client.host.requested() && client.host.status().ready;
        let (settings, guard, revision, ui_present) = {
            let mut state = handle.lock();
            let ready = allowed && state.snapshot.settings.enabled;
            if state.snapshot.ready && !ready {
                state.operations.cancel();
                state.operations = CancellationToken::new();
                state.revision = state.revision.wrapping_add(1);
                state.admission.cancel();
                state.admission = CancellationToken::new();
            }
            state.snapshot.ready = ready;
            (
                state.snapshot.settings.clone(),
                state.operations.clone(),
                state.revision,
                state
                    .ui_seen
                    .is_some_and(|at| at.elapsed() < Duration::from_secs(3)),
            )
        };
        if requested_revision != revision {
            for (_, _, task) in replies.drain(..) {
                task.abort();
            }
            requested_revision = revision;
            identity_attempted = false;
        }
        if event.as_ref().is_some_and(|event| !event.current(revision)) {
            continue;
        }
        if matches!(
            event.as_ref().map(|event| &event.event),
            Some(Push::Refresh)
        ) {
            identity_attempted = false;
            handle.lock().snapshot.error = None;
        }
        if allowed
            && settings.enabled
            && !identity_attempted
            && !identity_pending
            && jobs.len() < 8
            && handle.lock().snapshot.connect_id.is_empty()
        {
            identity_attempted = true;
            identity_pending = true;
            let current = client.clone();
            let guard = guard.clone();
            jobs.spawn(async move {
                let result = tokio::select! {
                    biased;
                    _=guard.cancelled()=>None,
                    result=current.host_assist_identity()=>Some(result),
                };
                Completion::Identity { revision, result }
            });
        }
        while answered
            .front()
            .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(120))
        {
            answered.pop_front();
        }
        let expired = {
            let mut state = handle.lock();
            if state
                .pending_room
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
            {
                state.snapshot.pending = None;
                state.pending_deadline = None;
                state.pending_room = None;
            }
            if !allowed
                || client.host.status().session_active
                || state
                    .attempt_deadline
                    .is_some_and(|at| Instant::now() >= at)
            {
                state.snapshot.attempt = None;
                state.attempt_deadline = None;
            }
            if state
                .pending_deadline
                .is_some_and(|at| Instant::now() >= at)
                || (!allowed && state.snapshot.pending.is_some())
            {
                state
                    .snapshot
                    .pending
                    .as_ref()
                    .filter(|p| !p.responding)
                    .map(|p| (p.id.clone(), p.token.clone(), state.pending_room.clone()))
            } else {
                None
            }
        };
        let mut events = std::collections::VecDeque::new();
        if let Some((id, token, room)) = expired {
            events.push_back((
                room,
                Push::Answer {
                    id,
                    token,
                    allow: false,
                },
            ));
        }
        if let Some(event) = event {
            events.push_back((event.room, event.event));
        }
        for (room, event) in events {
            let accept = allowed && settings.enabled;
            let mut outgoing = None;
            let mut answered_request = None;
            match event {
                Push::Refresh => (),
                Push::Reject(id) => {
                    outgoing = Some(Reply::Confirm(id.clone(), false, false));
                    answered.push_back((id, Instant::now()));
                }
                Push::Cancel(id) => {
                    // Drop in-flight HTTP futures, including any endpoint
                    // recovery, when this control request is withdrawn.
                    for (control_id, _, task) in &replies {
                        if control_id == &id {
                            task.abort();
                        }
                    }
                    let mut state = handle.lock();
                    if state.snapshot.pending.as_ref().is_some_and(|p| p.id == id) {
                        state.snapshot.pending = None;
                        state.pending_deadline = None;
                        state.pending_room = None;
                    }
                    if state.snapshot.attempt.as_ref().is_some_and(|p| p.id == id) {
                        state.snapshot.attempt = None;
                        state.attempt_deadline = None;
                    }
                    answered.push_back((id, Instant::now()));
                }
                Push::Mode(id) => {
                    if answered.iter().any(|(known, _)| known == &id) {
                        continue;
                    }
                    if accept && !client.host.status().session_active {
                        let mut state = handle.lock();
                        if state.snapshot.pending.is_none()
                            && !answered.iter().any(|(known, _)| known == &id)
                            && !state.snapshot.attempt.as_ref().is_some_and(|p| p.id == id)
                        {
                            state.snapshot.attempt = Some(Attempt {
                                id: id.clone(),
                                verifying: false,
                                expires_at: chrono::Utc::now().timestamp() + 30,
                            });
                            state.attempt_deadline = Some(Instant::now() + Duration::from_secs(30));
                        }
                    }
                    outgoing = Some(Reply::Mode(id, accept, settings.mode));
                }
                Push::Verify { id, salt } => {
                    if answered.iter().any(|(known, _)| known == &id) {
                        continue;
                    }
                    if let Some(attempt) = &mut handle.lock().snapshot.attempt {
                        if attempt.id == id {
                            attempt.verifying = true;
                        }
                    }
                    let code_available =
                        settings.mode.needs_password() && settings.validate().is_ok();
                    let (sign, backup) = if accept && code_available {
                        settings.verifiers(&salt)
                    } else {
                        (String::new(), None)
                    };
                    outgoing = Some(Reply::Sign(
                        id,
                        accept && code_available,
                        sign,
                        backup,
                        settings.mode,
                    ));
                }
                Push::Confirm { id, name } => {
                    if answered.iter().any(|(known, _)| known == &id) {
                        continue;
                    }
                    let mut state = handle.lock();
                    if state.snapshot.pending.as_ref().is_some_and(|p| p.id == id) {
                        continue;
                    }
                    // A confirmation can only be granted by a live user interface.
                    if !accept
                        || !ui_present
                        || state.snapshot.pending.is_some()
                        || crate::platform::capture::session_locked() != Some(false)
                    {
                        outgoing = Some(Reply::Confirm(id.clone(), false, false));
                        answered.push_back((id, Instant::now()));
                    } else {
                        state.snapshot.pending = Some(Confirmation {
                            id,
                            token: uuid::Uuid::new_v4().simple().to_string(),
                            name,
                            expires_at: chrono::Utc::now().timestamp() + 60,
                            responding: false,
                        });
                        state.snapshot.attempt = None;
                        state.attempt_deadline = None;
                        state.pending_deadline = Some(Instant::now() + Duration::from_secs(60));
                        state.pending_room = room.clone();
                    }
                }
                Push::Answer { id, token, allow } => {
                    let mut state = handle.lock();
                    if !state
                        .snapshot
                        .pending
                        .as_ref()
                        .is_some_and(|p| p.id == id && p.token == token && !p.responding)
                    {
                        continue;
                    }
                    let live = state.pending_deadline.is_some_and(|at| Instant::now() < at);
                    if let Some(pending) = &mut state.snapshot.pending {
                        pending.responding = true;
                    }
                    answered_request = Some(token);
                    outgoing = Some(Reply::Confirm(
                        id.clone(),
                        allow
                            && accept
                            && live
                            && ui_present
                            && crate::platform::capture::session_locked() == Some(false),
                        // Explicit local consent grants this request only. The
                        // configured password policy remains unchanged.
                        false,
                    ));
                    answered.push_back((id, Instant::now()));
                }
            }
            while answered.len() > 64 {
                answered.pop_front();
            }
            if let Some(reply) = outgoing {
                if jobs.len() >= 8 && answered_request.is_none() {
                    continue;
                }
                let current = client.clone();
                let guard = guard.clone();
                let (control_id, stage, granting) = reply.request();
                // A repeated push must not start a second copy while the
                // original reply is already recovering in the shared HTTP layer.
                if replies.iter().any(|(id, kind, task)| {
                    id == control_id && *kind == stage && !task.is_finished()
                }) {
                    continue;
                }
                let deadline = {
                    let state = handle.lock();
                    let expires = if granting && stage == "confirmation" {
                        state.pending_deadline
                    } else if granting
                        && state
                            .snapshot
                            .attempt
                            .as_ref()
                            .is_some_and(|a| a.id == control_id)
                    {
                        state.attempt_deadline
                    } else {
                        None
                    };
                    expires.unwrap_or_else(|| Instant::now() + Duration::from_secs(30))
                };
                let control_id = control_id.to_owned();
                let room = room.unwrap_or_default();
                let task = jobs.spawn(async move {
                    let result = tokio::select! {
                        biased;
                        _=guard.cancelled()=>return Completion::Cancelled,
                        _=room.cancelled()=>return Completion::Cancelled,
                        _=tokio::time::sleep_until(deadline.into())=>Err(anyhow::anyhow!("协助请求已过期")),
                        result=reply.send(&current)=>result,
                    };
                    Completion::Reply {
                        revision,
                        room,
                        request: answered_request,
                        result,
                    }
                });
                replies.push((control_id, stage, task));
            }
        }
    }
    jobs.abort_all();
    while jobs.join_next().await.is_some() {}
}

enum Reply {
    Mode(String, bool, Mode),
    Sign(String, bool, String, Option<String>, Mode),
    Confirm(String, bool, bool),
}
impl Reply {
    fn request(&self) -> (&str, &'static str, bool) {
        match self {
            Self::Mode(id, allowed, _) => (id, "mode", *allowed),
            Self::Sign(id, allowed, ..) => (id, "sign", *allowed),
            Self::Confirm(id, allowed, _) => (id, "confirmation", *allowed),
        }
    }
    async fn send(self, client: &HostClient) -> Result<()> {
        match self {
            Self::Mode(id, allowed, mode) => {
                client.host_assist_mode(&id, allowed, mode.wire()).await
            }
            Self::Sign(id, allowed, sign, backup, mode) => {
                client
                    .host_assist_sign(
                        &id,
                        allowed,
                        &sign,
                        backup.as_deref(),
                        mode.wire(),
                        mode.needs_confirmation(),
                    )
                    .await
            }
            Self::Confirm(id, allowed, password) => {
                client.host_assist_confirm(&id, allowed, password).await
            }
        }
    }
}

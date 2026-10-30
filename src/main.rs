#![allow(
    non_snake_case,
    reason = "The executable uses the OpenUUYC product name."
)]

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use openuuyc::account::api;
use openuuyc::account::client::AuthenticatedClient;
use openuuyc::account::login;
use openuuyc::application::app;
use openuuyc::diagnostics::logging;
use openuuyc::media;
use openuuyc::session::controller;

#[derive(Parser)]
#[command(
    name = "OpenUUYC",
    version,
    about = "OpenUUYC — 第三方 UU 远程协议兼容客户端"
)]
struct Cli {
    /// 系统按 `uuremote://` 协议注册拉起的协助链接
    #[arg(value_name = "URL", hide = true)]
    link: Option<String>,
    /// 日志过滤器，例如 info、debug、trace 或 openuuyc=trace
    #[arg(long, global = true)]
    log_level: Option<String>,
    /// 指定诊断日志文件（默认写入用户日志目录并自动轮转）
    #[arg(long, global = true)]
    log_file: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(hide = true)]
    DisplayRecovery { token: String },
    /// 列出或接上本机正在运行的被控终端会话
    Terminal {
        /// 会话名或编号；省略时列出
        session: Option<String>,
        /// 只列出会话，不接入
        #[arg(long)]
        list: bool,
        /// 新建一个会话并直接接上；位置参数作为名字
        #[arg(long)]
        new: bool,
    },
    #[command(hide = true)]
    PluginVideoHost,

    #[command(hide = true)]
    PluginHost { manifest: PathBuf },
    /// 打开完整图形设备中心
    Gui {
        #[arg(long)]
        background: bool,
        /// 初始码流帧率：auto、144、90、60 或 30
        #[arg(long)]
        fps: Option<media::FrameRateChoice>,
        /// 格式：auto、prefer-av1/prefer-hevc/prefer-h264；h264/h265/av1 为仅使用
        #[arg(long)]
        codec: Option<media::CodecPreference>,
        /// 是否优先使用平台原生硬件解码器
        #[arg(long, action = clap::ArgAction::Set)]
        hardware_decode: Option<bool>,
        /// 传输策略：auto、p2p 或 relay
        #[arg(long)]
        transport: Option<media::TransportChoice>,
    },
    /// 恢复保存的登录态，或进行二维码登录
    Login,
    /// 以适合脚本处理的文本格式列出设备
    Devices,
    /// 打开指定设备的观看窗口，关闭窗口或按 Ctrl+C 退出
    Connect {
        /// 设备的完整名称（必须唯一且完全匹配）
        #[arg(required_unless_present = "device_id", conflicts_with = "device_id")]
        device: Option<String>,
        /// 按已核实的设备 ID 选择目标，避免重复或变化的别名选错设备
        #[arg(long)]
        device_id: Option<String>,
        /// 静音启动，只影响本地播放
        #[arg(long)]
        mute: bool,
        /// 仅接收远端声音，不显示或控制画面
        #[arg(long)]
        audio_only: bool,
        /// 码流帧率：auto、144、90、60 或 30
        #[arg(long)]
        fps: Option<media::FrameRateChoice>,
        /// 格式：auto、prefer-av1/prefer-hevc/prefer-h264；h264/h265/av1 为仅使用
        #[arg(long)]
        codec: Option<media::CodecPreference>,
        /// 是否优先使用平台原生硬件解码器
        #[arg(long, action = clap::ArgAction::Set)]
        hardware_decode: Option<bool>,
        /// 传输策略：auto、p2p 或 relay
        #[arg(long)]
        transport: Option<media::TransportChoice>,
        /// 设备被残留会话占用时强制接管（断开现有连接）
        #[arg(long)]
        force_takeover: bool,
    },
}

fn main() -> Result<()> {
    openuuyc::application::bootstrap::initialize()?;
    let parsed = parse_cli();

    let cli = parsed.unwrap_or_else(|error| error.exit());
    let link = cli.link.clone();
    let command = cli.command.unwrap_or(Commands::Gui {
        background: false,
        fps: None,
        codec: None,
        hardware_decode: None,
        transport: None,
    });

    let _instance = if matches!(command, Commands::Gui { .. }) {
        let activation = match (link.as_deref(), &command) {
            // 链接是用户点出来的，哪怕这次是后台启动也得把窗口叫出来。
            (Some(uri), _) => app::instance::Activation::Link(uri),
            (
                None,
                Commands::Gui {
                    background: true, ..
                },
            ) => app::instance::Activation::Quiet,
            (None, _) => app::instance::Activation::Show,
        };
        match app::instance::acquire(activation)? {
            Some(instance) => Some(instance),
            None => {
                return Ok(());
            }
        }
    } else {
        None
    };

    let uninstalling = false;
    let _logging = if uninstalling {
        None
    } else {
        Some(logging::init(
            cli.log_level.as_deref(),
            cli.log_file.as_deref(),
        )?)
    };
    tracing::info!(target: "openuuyc", version = env!("CARGO_PKG_VERSION"), "application started");

    let result = match command {
        Commands::DisplayRecovery { token } => openuuyc::application::display_recovery(&token),
        Commands::Terminal { session, list, new } => {
            openuuyc::application::terminal_attach(session.as_deref(), list, new)
        }
        Commands::PluginVideoHost => openuuyc::plugins::video::host(),

        Commands::PluginHost { manifest } => openuuyc::plugins::host(&manifest),
        Commands::Gui {
            background,
            fps,
            codec,
            hardware_decode,
            transport,
        } => {
            let (mut options, startup_warning) = match media::preferences::load() {
                Ok(options) => (options, None),
                Err(error) => (
                    Default::default(),
                    Some(format!("读取编解码设置失败，暂用默认设置：{error:#}")),
                ),
            };
            if let Some(fps) = fps {
                options.frame_rate = fps;
            }
            if let Some(codec) = codec {
                options.codec = codec;
            }
            if let Some(hardware) = hardware_decode {
                options.decoder.mode =
                    media::selection::DecoderPreference::from_hardware(hardware).mode;
            }
            if let Some(transport) = transport {
                options.transport = transport;
            }
            app::run(app::GuiOptions {
                background,
                media: options,
                startup_warning,
                link,
            })
        }
        Commands::Login => {
            tokio::runtime::Runtime::new()?.block_on(login::interactive_login())?;
            Ok(())
        }
        Commands::Devices => tokio::runtime::Runtime::new()?.block_on(print_devices()),
        Commands::Connect {
            device,
            device_id,
            mute,
            audio_only,
            fps,
            codec,
            hardware_decode,
            transport,
            force_takeover,
        } => {
            let mut options = media::preferences::load()?;
            options.audio_only = audio_only;
            options.muted = mute;
            if let Some(fps) = fps {
                options.frame_rate = fps;
            }
            if let Some(codec) = codec {
                options.codec = codec;
            }
            if let Some(hardware) = hardware_decode {
                options.decoder.mode =
                    media::selection::DecoderPreference::from_hardware(hardware).mode;
            }
            if let Some(transport) = transport {
                options.transport = transport;
            }
            tokio::runtime::Runtime::new()?.block_on(connect_device(
                device,
                options,
                device_id,
                force_takeover,
            ))
        }
    };
    if let Err(error) = &result {
        tracing::error!(target: "openuuyc", error = %format_args!("{error:#}"), "application stopped with an error");
    }
    drop(_instance);

    result
}

/// 协议注册把链接当作第一个参数交给程序，所以根上有一个位置参数。子命令名
/// 要优先当子命令，否则 `OpenUUYC gui` 会被读成一条链接。
fn parse_cli() -> std::result::Result<Cli, clap::Error> {
    use clap::{CommandFactory, FromArgMatches};
    let args: Vec<_> = std::env::args_os().collect();
    let has_link = args.iter().skip(1).any(|a| {
        a.to_string_lossy()
            .to_ascii_lowercase()
            .starts_with("uuremote:")
    });
    // An OS protocol activation is a single argument, never a second command.
    // Clap diagnostics must not echo an invite or its verification code.
    if has_link && args.len() != 2 {
        return Err(clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            "协助链接启动参数无效",
        ));
    }
    let matches = Cli::command()
        .subcommand_precedence_over_arg(true)
        .try_get_matches_from(args)
        .map_err(|error| {
            if has_link {
                clap::Error::raw(clap::error::ErrorKind::InvalidValue, "协助链接启动参数无效")
            } else {
                error
            }
        })?;
    Cli::from_arg_matches(&matches)
}

async fn connect_device(
    device: Option<String>,
    options: media::ConnectionMediaOptions,
    device_id: Option<String>,
    force_takeover: bool,
) -> Result<()> {
    controller::run_saved_viewer_window(
        device.unwrap_or_else(|| device_id.clone().expect("clap requires a target")),
        options,
        device_id,
        force_takeover,
    )
    .await
}

async fn print_devices() -> Result<()> {
    let client = AuthenticatedClient::from_saved_session()?;
    let devices = client.list_devices().await;
    client.close().await;
    let devices = devices?;

    println!("当前虚拟设备：");
    print_device(None, &devices.current_device);
    println!("\n我的设备：");
    for (index, device) in devices.my_binded_devices.iter().enumerate() {
        print_device(Some(index + 1), device);
    }
    if devices.my_binded_devices.is_empty() {
        println!("  （无）");
    }
    Ok(())
}

fn print_device(index: Option<usize>, device: &api::DeviceInfo) {
    let prefix = index.map_or_else(|| "  ".to_owned(), |value| format!("  [{value}] "));
    println!(
        "{prefix}{} [{}] — {}，{}，可控 {}，会话参与者 {}",
        device.alias,
        device.device_id,
        device.status_label(),
        device.platform_label(),
        if device.controlled_support && device.controllable {
            "是"
        } else {
            "否"
        },
        device.participant_count()
    );
}

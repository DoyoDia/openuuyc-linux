# OpenUUYC Linux

![OpenUUYC](assets/banner.png)

这是用 Rust 编写的 UU 远程第三方 Linux 客户端，支持主控、被控和远程协助。本仓库仅维护 Linux 构建，不包含 Windows/macOS 后端、Windows 安装器和内核驱动。

Linux 客户端仍保留与官方客户端及 Windows 远端互通所需的协议、文件路径、键码和剪贴板格式；移除本地 Windows 后端不等于移除这些协议能力。

## 下载与使用

使用本仓库 Linux CI 的 deb 或压缩包产物，或按下文从源码构建。扫码或短信登录后连接设备；也可在登录页开启“接受远程协助”。需要本地图形会话，不能接管 Linux 登录界面。

## Linux 功能矩阵

以下沿用本分支已有的功能记录，实际能力取决于桌面、显卡、驱动、权限和远端。✅ 支持，⚠️ 部分支持或有前提，❌ 不支持，— 该角色不涉及。

| 功能 | Linux 主控 | Linux 被控（Xorg） | Linux 被控（Wayland） |
| --- | --- | --- | --- |
| 登录方式 | ✅ 扫码 / 短信 | ✅ 未登录可游客协助 | ✅ 未登录可游客协助 |
| 同账号连接 | ✅ 可同时连多台 | ✅ 需开“允许被控” | ✅ 需开“允许被控” |
| 设备 ID / 验证码协助 | ✅ 含最近与收藏 | ✅ 账号及游客 | ✅ 账号及游客 |
| 协助邀请链接 `uuremote://` | ✅ 经 .desktop 登记 | — | — |
| 画面采集 | — | ✅ NvFBC → MIT-SHM → 门户 | ⚠️ wlr-screencopy → 门户，需 XWayland；缩放桌面需 XWayland ≥ 23.1 |
| 硬件编码 | — | ✅ NVENC / VA-API H.264 | ✅ NVENC / VA-API H.264 |
| 软件编码 | — | ✅ H.264 4:2:0 | ✅ H.264 4:2:0 |
| 硬件解码 | ⚠️ 仅 VA-API H.264 | — | — |
| 软件解码 | ⚠️ H.264；AV1 仅 8 位 4:2:0 | — | — |
| 指定显卡 / GPU 调度 | ⚠️ 自动（桌面所在显卡） | ⚠️ 自动，无 GPU 调度 | ⚠️ 自动，无 GPU 调度 |
| H.265 | ❌ | ⚠️ 仅 NVENC | ⚠️ 仅 NVENC |
| AV1 | ⚠️ 软解 8 位 4:2:0 | ⚠️ NVENC 4:2:0，需 RTX 40 系及以上 | ⚠️ NVENC 4:2:0，需 RTX 40 系及以上 |
| 10 位 / HDR | ❌ | ❌ | ❌ |
| 4:4:4 真彩 | ⚠️ H.264 软解，≤1080p | ⚠️ 仅 NVENC | ⚠️ 仅 NVENC |
| 键盘鼠标 | ✅ | ✅ XTest，免设置 | ⚠️ uinput 需权限，否则仅 XWayland |
| 手柄与震动 | ✅ 最多四个标准手柄，连接支持手柄的官方被控端 | ❌ | ❌ |
| 绝对 / 相对鼠标 | ✅ 游戏锁定、可选节流 | ✅ | ⚠️ 绝对定位需 xdg-output |
| 触摸（多点） | ❌ 只发键鼠 | ⚠️ 单指模拟 | ⚠️ 单屏未旋转时多点 |
| 文字输入（移动端输入法） | — | ✅ XTest | ⚠️ 虚拟键盘协议，GNOME 退回 XTest |
| 拦截本机快捷键 | ⚠️ Xorg 抓键盘 / Wayland 需合成器协议 | — | — |
| 多显示器切屏 | ✅ 顶栏屏标签，跟随远端拓扑 | ✅ | ✅ 门户需共享多屏 |
| 播放中切换设备 | ⚠️ 顶栏与全屏浮动栏，未实测 | — | — |
| 多屏独立窗口 | ⚠️ 标签拖出，独立窗口各一块屏；仅硬解，Wayland 摆位由合成器决定 | — | — |
| 分辨率 / DPI 调整 | ✅ 远端设置、本地拉伸 | ⚠️ RandR 分辨率，无 DPI | ⚠️ 物理屏保持原分辨率（XWayland 不能改，按拒绝处理），虚拟屏经 niri；无 DPI |
| 虚拟屏 / 超级屏 / 无屏兜底 | ✅ 操作远端 | ❌ | ⚠️ 仅 niri：虚拟屏与无屏兜底，内核 ≥ 6.19，需启用 openuuyc-vkms |
| 桌面声音 | ✅ Pulse / PipeWire / ALSA | ✅ 录监听源 | ✅ 录监听源 |
| 仅音频连接 | ✅ | ✅ | ✅ |
| 麦克风 / 虚拟声卡 | ✅ 发送麦克风 | ❌ 无虚拟声卡 | ❌ 无虚拟声卡 |
| 剪贴板文字 / 图片 / 文件 | ⚠️ 无富文本，文件经 FUSE | ⚠️ 无富文本，文件经 FUSE | ⚠️ 同左，图片可能到不了 XWayland |
| 剪贴板同步开关（标题栏） | ✅ | — | — |
| 文件拖放（拖入 / 拖出 / 拖回） | ⚠️ 拖入/拖出，无拖回 | ⚠️ 拖入/拖出，无拖回 | ⚠️ 仅拖出（需 layer-shell） |
| 向官方主控发文件（队列浮窗） | — | ✅ | ❌ |
| 独立文件传输 | ✅ | ✅ 根为 `\`，路径对外用反斜杠 | ✅ 同左 |
| 端口转发 | ✅ TCP / UDP | ✅ 需开许可 | ✅ 需开许可 |
| 被控终端（UU 手机端的「终端」） | ❌ 不提供 | ✅ 需开许可，以桌面用户身份运行；「终端会话」页可本机新建、用系统终端接上；本机开的会话另需「允许主控接入本机终端会话」才对主控可见 | ✅ 同左 |
| 批注 / 白板 | ❌ | ❌ 拒绝请求 | ❌ 拒绝请求 |
| 远程关机 / 重启 | ✅ | ✅ logind，受 polkit 约束 | ✅ logind，受 polkit 约束 |
| 远程开机（WoL） | ✅ | ⚠️ 网卡需手动 ethtool | ⚠️ 网卡需手动 ethtool |
| 息屏点亮与防休眠 | ✅ 观看时防休眠 | ✅ DPMS 点亮 | ⚠️ 视桌面，Hyprland 等不能点亮 |
| 锁屏 / 登录界面被控 | — | ❌ 无常驻服务 | ❌ 无常驻服务 |
| 开机自启 | ✅ 登录后进托盘 | ⚠️ 登录桌面后才能被控 | ⚠️ 登录桌面后才能被控 |
| 托盘与被控红角标 | ✅ SNI，GNOME 需扩展 | ✅ | ✅ |
| 访问通知 | — | ✅ 浮窗 / 桌面通知 | ✅ 浮窗位置由合成器定 |
| 性能监控 / 异常图标 / 全屏操作栏 | ✅ | — | — |
| 设备资料与壁纸 | — | ⚠️ 壁纸仅部分桌面 | ⚠️ 壁纸仅部分桌面 |
| 诊断包（本机导出 / 远端获取） | ⚠️ 无解码检查页 | ✅ | ✅ |
| 插件与节点图 | ⚠️ 可管理插件，播放器无节点图 | — | — |
| 1:1 实际像素 | ❌ 按钮置灰 | — | — |
| 自定义 / 全局快捷键 | ⚠️ 仅播放窗口有焦点时 | — | — |
| 凭据存储 | ✅ Secret Service，无明文回退 | ✅ 同左 | ✅ 同左 |
| 安装服务 / 驱动 | ❌ 用 deb / AUR 包 | ❌ | ❌ |
| 更新 | ⚠️ 仅提醒，指向上游 Release | ⚠️ 同左 | ⚠️ 同左 |

## 构建与安装

需要 Rust stable（edition 2024）与 C/C++ 工具链。

### Arch / Omarchy

[`packaging/aur/`](packaging/aur/README.md) 里有两份 AUR 配方：`openuuyc-bin` 安装 Release 里的压缩包，`openuuyc-git` 跟踪 `linux-port` 分支源码构建。它们还没有发布到 AUR，可以直接在本地构建：

```bash
cd packaging/aur/openuuyc-bin && makepkg -si
```

Omarchy 用户可以把 [`packaging/omarchy/omarchy-menu.jsonc`](packaging/omarchy/omarchy-menu.jsonc) 里的条目加进 `~/.config/omarchy/extensions/omarchy-menu.jsonc`，从 `Super + Space → Install → Service` 安装。不用 AUR 时，在源码目录构建后（或解压 Release 里的 `OpenUUYC-<版本>-x86_64-linux.tar.zst` 后）运行 `./install-omarchy.sh`，它会安装程序、桌面入口、uinput 规则和 Omarchy 菜单项（`--uninstall` 卸载）。

自建：

```bash
sudo pacman -S --needed base-devel cmake clang nasm pkg-config rust \
    libva libx11 libxcb libxrandr libxi libxcursor libxfixes \
    libxkbcommon libxkbcommon-x11 wayland libglvnd vulkan-icd-loader \
    dbus alsa-lib openssl fuse3 noto-fonts-cjk
cargo build --release --bin OpenUUYC
```

### Debian / Ubuntu

Ubuntu 22.04 / Debian 12 及以上：

```bash
sudo apt install build-essential cmake clang nasm pkg-config libva-dev \
    libasound2-dev libdbus-1-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libwayland-dev libx11-dev libxcb1-dev libxrandr-dev libxi-dev libxcursor-dev \
    libgl1-mesa-dev libvulkan-dev libudev-dev libssl-dev fonts-noto-cjk fuse3
git clone https://github.com/djkcyl/openuuyc.git
cd openuuyc
cargo build --release
./target/release/OpenUUYC gui
```

### 被控端虚拟显示器（可选）

主控可以要求被控端凭空多出一块显示器。写内核的 configfs 只有 root 能做，所以包里另带一个小程序 `openuuyc-vkms` 和一份套接字激活的系统服务：平时不跑，第一次用到时由套接字拉起。条件：

- **Linux 6.19 或更新的内核**，且编译了 `CONFIG_DRM_VKMS`。更老的内核上 VKMS 没有 configfs 接口（`modinfo -F parm vkms` 里没有 `create_default_dev` 就是），程序不会去碰它，诊断页会写「内核不支持」；
- systemd（套接字激活）；
- 当前合成器能把新输出接进桌面布局，**目前只有 niri**。其他桌面（GNOME、KDE、sway、Xorg……）上主控不会出现「添加虚拟屏」这个入口。

**无屏兜底**：被控机一块启用的输出都没有时（显示器没接或没启用），连接时会自动接一块兜底屏，条件同上（niri、内核 ≥ 6.19、已启用 `openuuyc-vkms.socket`）；不满足时照旧连不上，并报「本机没有可用的显示器，也无法自动添加兜底屏」和具体原因。兜底屏断线保留：有连接在时，别的屏接入并出帧后由连接回收；没有连接时，别的屏启用满 2 秒就收掉。进程退出时随辅助程序收回，退出过程中不会再新建。它和主控添加的虚拟屏一样占用辅助程序给每个用户的 4 条连接中的 1 条。

deb 与 AUR 包都会装上它，但**不会替你启用**：不启用一切照旧，只是诊断页里「虚拟显示器」那一行显示「已安装，服务未启用」。启用：

```bash
sudo systemctl enable --now openuuyc-vkms.socket
```

从源码构建或手动安装时，把 `crates/vkms` 编出来的 `openuuyc-vkms` 装到 `/usr/bin/`、两份单元装到 `/usr/lib/systemd/system/`，再执行上面那条命令：

```bash
(cd crates/vkms && CARGO_TARGET_DIR=../../target cargo build --release --locked)
sudo install -Dm755 target/release/openuuyc-vkms /usr/bin/openuuyc-vkms
sudo install -Dm644 packaging/linux/openuuyc-vkms.socket \
    packaging/linux/openuuyc-vkms.service /usr/lib/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now openuuyc-vkms.socket
```

### 被控端输入权限（Wayland 必需）

Wayland 会话下键鼠经 `/dev/uinput` 注入。安装包会带上 udev 规则，重载后当前登录用户即可使用：

```bash
sudo udevadm control --reload-rules && sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/uinput.conf
```

规则使用 `TAG+="uaccess"`，由 systemd-logind 给本机活动会话授权，**不需要把用户加进 `input` 组，也不需要重新登录**。从源码构建或手动安装时，把
[`packaging/linux/60-openuuyc-uinput.rules`](packaging/linux/60-openuuyc-uinput.rules)
放到 `/etc/udev/rules.d/` 即可。没有权限时程序会记录一条警告并回退到 XTest（只影响 XWayland 客户端）。Xorg 会话不使用 uinput，无需这一步。

Wayland 下被控先用合成器自己的 wlroots screencopy（Sway、Hyprland、niri 等都提供），不需要门户，也不弹授权。合成器不提供或无法复制画面时（GNOME、KDE 等）改用 XDG 屏幕共享门户：

```bash
# GNOME / KDE
sudo pacman -S --needed xdg-desktop-portal-gnome   # KDE 为 xdg-desktop-portal-kde
# wlroots 系合成器也走门户时（如 OPENUUYC_CAPTURE=portal）：Hyprland 为 xdg-desktop-portal-hyprland，Sway 等为 xdg-desktop-portal-wlr
```

经门户采集时，首次被控会弹出一次共享授权，之后记在 `~/.local/share/OpenUUYC/screencast-restore-token`。

### 硬件解码 / 编码

- **解码**：H.264 走 VA-API。AMD 用 `mesa`，Intel 用 `intel-media-driver`，NVIDIA 用 `libva-nvidia-driver`（Ubuntu 为 `nvidia-vaapi-driver`；并设 `NVD_BACKEND=direct`）；libva 2.15 以前认不出 NVIDIA 显卡对应的驱动，程序会自行指定，无需设置 `LIBVA_DRIVER_NAME`。
- **编码**（被控）：NVIDIA 用 NVENC；AMD 与 Intel 经 VA-API 编码 H.264（Intel 需 `intel-media-driver` 或 `libva-intel-driver`，AMD 用 `mesa`），都不可用时回退软件编码。
- **老 Intel GPU / 虚拟机**：Haswell/Broadwell 的 Vulkan 驱动未通过一致性认证，程序仍会使用它（设 `WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER=0` 恢复 wgpu 默认的隐藏行为）；驱动无法向 Wayland surface 呈现时，可安装 `vulkan-swrast`（lavapipe）并用 `OPENUUYC_BACKEND=gl` 或 `OPENUUYC_BACKEND=vulkan` 指定界面后端。

### 运行前提与开关

- **运行环境**：登录态存在系统密钥环（Secret Service，需要 gnome-keyring、KWallet 等，没有明文回退）。托盘用 StatusNotifierItem，GNOME 需要 AppIndicator 扩展（Ubuntu 默认启用），桌面没有托盘时关闭窗口即退出；面板比程序晚起来时每 2 秒重试，托盘一出现就改回关闭收进托盘。粘贴远端文件要 `fuse3`。Wayland 会话被控也要有 XWayland（显示器列表与输入注入器都先连 X 服务器）；缩放（含分数缩放）的桌面上，门户共享的屏幕按合成器 `xdg-output` 布局里的输出名与所选显示器配对，需要 XWayland 23.1 及以上（输出以连接器命名）且程序能拿到 `WAYLAND_DISPLAY`，配不上时报错，不会采错屏。拖出文件的预览图要 `gio` 与 `rsvg-convert`。
- **开机自启**：桌面接进 `graphical-session.target` 时是 systemd 用户单元 `app-openuuyc.service`（`journalctl --user -u app-openuuyc` 看日志）。单元名与 `openuuyc.desktop` 对应，门户才认得出这是 OpenUUYC，KDE 记下的屏幕共享授权才能沿用；旧版本启用的 `openuuyc.service` 在程序启动时迁移过去，迁移前的那次登录由它转发。否则写 `~/.config/autostart/openuuyc.desktop`。两者都不处理的合成器（无 uwsm 的 sway、Hyprland 等）请自行启动 `OpenUUYC gui --background`。再次启动程序会经 `$XDG_RUNTIME_DIR/openuuyc/control.sock` 唤起已运行的控制中心。
- **协助邀请链接**：安装包的 `openuuyc.desktop` 登记了 `x-scheme-handler/uuremote`，没有 Windows 那样的接管开关，可用 `xdg-mime default <程序>.desktop x-scheme-handler/uuremote` 改。便携运行时把链接作为唯一参数传给程序。
- **被控终端**：默认关闭，在被控设置的访问权限里打开，打开时会确认一次；以 root 运行时不提供，本机也不能新建。shell 跑在 pty 上，断线后保留、重连可接回（重开同一会话即接管，旧连接不再收输出），接回时重画当前屏幕，不带回已滚出屏幕的输出；主控开的会话在退出或切换账号、关闭终端开关、禁止被控或在本机断开连接时结束，两端都没有附着 30 分钟后也会结束。设置页显示运行中的会话数，可一键结束全部。
  - **终端会话页**：侧边栏常驻，列出全部会话，可结束；也能在本机新建会话，或用系统终端接上任一会话（`$TERMINAL` 优先，再找常见的终端模拟器）。命令行是 `OpenUUYC terminal [编号或名字]`、`--list`、`--new [名字]`，控制中心没在跑时新建和接入会以 `gui --background` 把它拉起来。本机经 `$XDG_RUNTIME_DIR/openuuyc/terminal.sock` 接入，只有同一用户能连。本机与主控同时附着时输入输出同步，终端按两边较小的尺寸显示；关掉本地终端只断开这一端。本机开的会话不会因空闲被结束，shell 退出后从列表消失。
  - **本机会话对主控的可见性**：本机开的会话默认不对主控可见，主控看不到、也接不上。要让主控接入，在「终端」下再打开「允许主控接入本机终端会话」，打开时会再确认一次，关闭终端开关时它一并关闭。主控接入本机会话时访问通知会写明；关掉这一项后主控立即断开这些会话，shell 留给本机。
- **远程开机**：配置网卡需要 root，程序内不提供，请用 `sudo ethtool -s <网卡> wol g`。远程关机与重启经 systemd-logind，受 polkit 约束。
- **更新**：「关于」页的更新提醒指向上游的 Windows Release；Linux 版请从本 fork 的 Release 或 deb / AUR 包更新。下面的「命令行更新」只适用于 Windows。
- **Gamescope**：在 SteamOS 游戏模式中，将原生程序添加为非 Steam 游戏即可，无需指定兼容性工具。启动时处理显示连接与缩放，控制中心匹配当前屏幕尺寸；保留手动设置的缩放比例。登录凭据使用系统配置的 Secret Service，锁定时先请求解锁；KWallet 解锁窗口会关联到当前 Steam 应用。
- **手柄**：Linux 主控通过系统 SDL2 2.0.6 及以上读取标准手柄（Arch 安装 `sdl2-compat`，Debian / Ubuntu 安装 `libsdl2-2.0-0`），支持双摇杆、十字键、按钮和双扳机，震动回传还需要 SDL2 2.0.9 及以上版本和兼容手柄。SteamDeck 的 Steam Input 选择手柄布局（例如「支持视角控制的手柄」）。开启远控输入、将焦点留在播放窗口并松开已有按键后开始转发；失焦、操作本地菜单、拔出或断开连接时释放远端状态并停止震动。
- **环境变量**：`OPENUUYC_CAPTURE` 指定采集后端顺序（如 `nvfbc`、`wlr`、`x11,portal`）；`OPENUUYC_ZEROCOPY=1` 让 VA-API 解码画面以 dmabuf 直接交给渲染器（只在 Intel 上验证过）；`OPENUUYC_CAPTURE_DMABUF=1` 与 `OPENUUYC_GPU_CONVERT=1` 是门户采集的试验开关；`OPENUUYC_BACKEND=gl|vulkan` 指定界面后端。

[`devtools/probes/`](devtools/probes/README.md) 里是验证 Linux 零拷贝与 GPU 颜色转换可行性的开发用探测程序（C 与一个独立的 Rust 小工程），不参与构建，用于在新驱动或新显卡上先确认 dmabuf 导出、Vulkan 导入和 VA-API 编码是否可用。

Arch / EndeavourOS 的构建依赖：

```bash
sudo pacman -S --needed base-devel rust cmake nasm clang pkgconf libva alsa-lib dbus \
    libxkbcommon libxkbcommon-x11 wayland libx11 libxcb libxrandr libxi \
    libxcursor libglvnd vulkan-icd-loader openssl fuse3 noto-fonts-cjk
```

打包为 deb（GitHub Actions 的 “Linux deb” 工作流在 Ubuntu 22.04 上分别为 amd64 与 arm64 原生构建同样的包，产物见运行页面的 Artifacts，打 `v*` 标签时附到 Release。arm64 包只做了编译与安装检查，没有在 ARM 机器上实际运行过；Release 里的压缩包与单文件程序只有 x86_64）：

```bash
cargo install cargo-deb
cargo build --release --locked --bin OpenUUYC
(cd crates/vkms && CARGO_TARGET_DIR=../../target cargo build --release --locked)
cargo deb --no-build
sudo apt install ./target/debian/openuuyc_*.deb
```

安装后从应用菜单启动，或运行 `OpenUUYC`。

## 开发验证

```bash
cargo check --all-targets --locked
cargo test --lib --locked
cargo test --manifest-path crates/vkms/Cargo.toml --locked
cargo test --manifest-path devtools/posix-pty-check/Cargo.toml --locked
```

本地套接字、PTY 和硬件相关测试需要相应的系统权限或设备。Linux 播放窗口使用 wgpu；Windows 专用 D3D11 插件示例不在本仓库中，节点图/SDK 数据结构仍保留供管理界面使用。

## 反馈

问题和建议请提交到 [Issues](https://github.com/djkcyl/openuuyc/issues)。报告问题时附上双方版本、操作步骤、大致发生时间及导出的诊断包，分享前检查是否含私人信息。

## 鸣谢

感谢以下团队、项目和贡献者为 OpenUUYC 提供的参考、技术基础与开发支持：

- [网易 UU 远程](https://uuyc.163.com/)：本项目的协议兼容目标，也是远程连接功能与交互行为的参考。
- [FFmpeg](https://ffmpeg.org/)：视频编解码基础算法及硬件解码适配的重要来源。
- [rav1d](https://github.com/memorysafety/rav1d) / dav1d：AV1 软件解码核心及 SIMD 实现。
- [Cisco OpenH264](LICENSE)：软件编码器的码率控制、屏幕变化检测与运动搜索等算法参考。
- [WebRTC](https://webrtc.googlesource.com/src/+/refs/branch-heads/5481/)、[WebRTC-rs](https://github.com/webrtc-rs/webrtc) 与 [goog_cc](https://github.com/kixelated/goog_cc)：实时传输、音频接收处理与拥塞控制基础。
- Opus / [SpeexDSP](src/media/audio/COPYING.SpeexDSP)：音频编解码与重采样算法。
- egui / egui-wgpu / wgpu：图形界面与渲染基础。
- [OpenAI](https://openai.com/)：Codex 为本项目的开发提供 AI 编程辅助。
- [sisi0318](https://github.com/sisi0318)：提供两个 Codex x20 账号，支持本项目开发。

也感谢[项目贡献者](https://github.com/djkcyl/openuuyc/graphs/contributors)及提供测试反馈和改进建议的用户。

## 许可

OpenUUYC 非网易官方项目。源码公开，但项目整体未采用开源许可证，使用与分发条件见 [LICENSE](LICENSE)。

第三方及派生代码保留各自的许可，详见 [NOTICES](NOTICES.md)。

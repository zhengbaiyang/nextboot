# NextBoot — 跨平台 UEFI 多系统重启切换工具

**NextBoot** 是一款面向 **Windows 10/11** 与 **Ubuntu 22.04 / 24.04** 双系统用户的轻量级多系统切换工具（支持桌面 **GUI** 与同源 **CLI**），专门解决远程控制双系统电脑时的重启切换与防掉回痛点。

## 解决的核心痛点

当工作电脑安装了 **Ubuntu + Windows 双系统** 且主引导默认为 Ubuntu 时：
- 在远控软件（RustDesk / ToDesk / 向日葵 / RDP 等）下重启电脑，无法在主板/GRUB 启动菜单手工按键选系统；
- 从 Ubuntu 切到 Windows 后，Windows 系统更新或意外自动重启会再次掉回默认的 Ubuntu，导致远控目标失联。

**NextBoot** 通过直接读写主板 **UEFI NVRAM** 解决上述问题，Windows 与 Ubuntu 双端均**不需要互相挂载或修改对方磁盘分区**，零侵入、安全可靠。

---

## 核心特性

- **双端统一 UEFI NVRAM 引擎**
  - **Windows**：原生 Win32 API（`GetFirmwareEnvironmentVariableW` / `SetFirmwareEnvironmentVariableW` + `SeSystemEnvironmentPrivilege`）。
  - **Ubuntu**：原生读写 `/sys/firmware/efi/efivars`（自动处理 `FS_IMMUTABLE_FL` 只读标志），与 Windows 共用同一套 UEFI `EFI_LOAD_OPTION` 二进制解析器，零外部工具依赖。
- **固件模式自动检测**：启动时自动检测是否为 UEFI；遇到旧版 Legacy BIOS (MBR) 会给出明确提示。
- **两种切换模式**
  1. **仅下一次重启生效（`BootNext`）**：指定下一次重启进入目标系统，消费一次后自动恢复原默认顺序；支持一键取消指定。
  2. **设为长期默认（修改 `BootOrder`）**：将目标系统置顶为长期默认启动项，避免 Windows 多次更新重启后掉回 Ubuntu。
  - 以上两种模式均支持「**仅保存设置，稍后重启**」和「**保存并立即重启**」。
- **智能筛选与远控安全保护**
  - 自动识别 Windows / Ubuntu / Linux 等操作系统并展示对应图标；
  - 默认隐藏网卡 PXE、U 盘、光驱、UEFI Shell 等非系统杂项，防止远控误点失联（提供「**显示全部 UEFI 项（高级）**」开关）；
  - 立即重启前弹出二次确认框，内置「**强制重启（忽略未保存程序阻塞 / systemd 抑制锁）**」可选项（默认不勾选，远控怕卡在重启界面时可一键勾选）。
- **按需提权，零后台常驻**
  - 按需打开、用完即关，不常驻后台服务和托盘。
  - **Windows**：Release 版嵌入 `requireAdministrator` 清单，打开时弹一次 UAC，后续查看/写入/重启免重复弹窗。
  - **Ubuntu**：GUI 以普通用户运行（完美兼容 X11 与 Wayland），查看状态免密；写入或重启时通过 `pkexec` 单次调用 `nextboot-cli`，全程仅输一次密码。
- **内置模拟模式（`--mock`）**：可在不触碰真实主板 NVRAM、不真正重启的前提下安全测试 GUI 与 CLI。

---

## 工程架构

```text
.
├── crates/
│   ├── boot-core/          # 跨平台 UEFI NVRAM 解析/写入、智能 OS 分类、重启控制、Mock 引擎
│   └── boot-cli/           # 独立命令行工具 nextboot-cli（兼任 Ubuntu GUI 的 pkexec 提权执行器）
├── src-tauri/              # Tauri v2 桌面宿主（Windows UAC 清单、Linux pkexec 封装）
├── src/                    # React + TypeScript + Vite 前端桌面界面
└── .github/workflows/      # GitHub Actions Windows + Ubuntu 双端自动测试与打包工作流
```

---

## 命令行（CLI）使用说明

在 Ubuntu SSH 终端或 Windows 管理员终端下均可直接使用 `nextboot-cli`：

```bash
# 查看当前运行系统、长期默认与下次指定状态
nextboot-cli status

# 列出操作系统启动项（默认智能过滤 PXE/U盘等；加 --all 查看全部）
nextboot-cli list
nextboot-cli list --all

# 设置仅下一次重启进入 Windows（支持编号 0002 或关键词 windows / ubuntu）
sudo nextboot-cli next windows

# 设置下次重启进入 Windows 并立即重启（加 --force 忽略未保存程序/阻止关机锁）
sudo nextboot-cli next windows --reboot
sudo nextboot-cli next windows --reboot --force

# 取消已设置的下次重启指定 (BootNext)
sudo nextboot-cli clear-next

# 将 Windows 设为长期默认启动项（防止 Windows 自动更新多次重启掉回 Ubuntu）
sudo nextboot-cli default windows
sudo nextboot-cli default windows --reboot

# 所有命令均支持 --mock（模拟测试）与 --json（脚本/程序解析）
nextboot-cli --mock status
nextboot-cli --mock list --json
```

> **Windows 提示**：Windows 下读写 UEFI 变量需要管理员权限，请在「以管理员身份运行」的 PowerShell / CMD 中执行 `nextboot-cli.exe`（`--mock` 模式无需管理员权限）。

---

## 本地开发与构建

### 环境依赖
- **Rust**（1.80+，stable 工具链）
- **Node.js**（20+）与 `npm`
- **Ubuntu 构建依赖**（Ubuntu 22.04 / 24.04）：
  ```bash
  sudo apt-get update
  sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev build-essential
  ```

### 运行单元测试与模拟模式

```bash
# 运行核心库与 CLI 单元测试
cargo test -p boot-core -p boot-cli

# 模拟模式体验 CLI
cargo run -p boot-cli -- --mock list
cargo run -p boot-cli -- --mock next windows
cargo run -p boot-cli -- --mock status

# 安装前端依赖
npm install

# 启动桌面端开发模式（使用模拟 UEFI 数据，安全调试界面）
NEXTBOOT_MOCK=1 npm run tauri dev
# PowerShell:
# $env:NEXTBOOT_MOCK="1"; npm run tauri dev
```

### 正式打包

```bash
# 1. 先构建 Release 版 CLI（Linux 打 .deb 时会自动将 nextboot-cli 打包到 /usr/bin/nextboot-cli）
cargo build --release -p boot-cli

# 2. 构建桌面应用与安装包（Windows 生成 .exe / NSIS / MSI，Ubuntu 生成 .deb）
npm run tauri build
```

---

## 常见问题排查

1. **提示当前不是 UEFI 引导模式**
   - Windows 下运行 `msinfo32` 检查「BIOS 模式」是否为 `UEFI`；
   - Ubuntu 下检查 `/sys/firmware/efi/efivars` 是否存在。
   - NextBoot v1 仅支持 UEFI 机器，暂不支持传统 Legacy BIOS (MBR)。
2. **Ubuntu 下 GUI 点击切换提示找不到 `pkexec` 或 `nextboot-cli`**
   - 通过正式 `.deb` 包安装时会自动安装 `/usr/bin/nextboot-cli`；
   - 本地源码开发运行 GUI 时，请先执行 `cargo build -p boot-cli` 确保 `nextboot-cli` 与主程序位于同一输出目录，或将其放入 `PATH`。
3. **如何验证是否支持修改主板 NVRAM**
   - 可在 Ubuntu 下尝试 `sudo nextboot-cli next <id>` 后执行 `nextboot-cli status` 确认 `下次指定` 已生效，最后用 `sudo nextboot-cli clear-next` 复原。

## License

MIT

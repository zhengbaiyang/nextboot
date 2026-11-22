use boot_core::{
    create_backend, format_boot_id, resolve_target_id, BootEntry, BootError, BootState,
    FirmwareType, RebootOptions,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "nextboot-cli",
    version,
    about = "NextBoot - 跨平台 UEFI 多系统重启切换命令行工具 (Windows / Ubuntu)"
)]
struct Cli {
    /// 使用模拟 UEFI 数据（不修改本机真实 NVRAM，不真正重启）
    #[arg(long, global = true)]
    mock: bool,

    /// 以 JSON 格式输出结果（便于脚本与 GUI 提权调用解析）
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 查看当前 UEFI 状态、默认启动项和下次指定项
    Status,

    /// 列出启动项（默认智能过滤 PXE/U 盘等杂项）
    List {
        /// 显示全部 UEFI 启动项（包含网卡 PXE、U 盘、固件工具等）
        #[arg(short, long)]
        all: bool,
    },

    /// 设置「仅下一次重启」进入目标系统 (UEFI BootNext)
    Next {
        /// 目标启动项编号（如 0002 / Boot0002）或系统关键词（如 windows / ubuntu）
        target: String,

        /// 设置完成后立即重启电脑
        #[arg(short, long)]
        reboot: bool,

        /// 强制重启（忽略未保存程序阻塞或 systemd 抑制锁，需配合 --reboot）
        #[arg(short, long)]
        force: bool,
    },

    /// 清除已设置的「下次启动指定 (BootNext)」，恢复按默认顺序启动
    ClearNext,

    /// 将目标系统设为「长期默认启动项」(置顶 UEFI BootOrder)
    Default {
        /// 目标启动项编号（如 0002 / Boot0002）或系统关键词（如 windows / ubuntu）
        target: String,

        /// 设置完成后立即重启电脑
        #[arg(short, long)]
        reboot: bool,

        /// 强制重启（忽略未保存程序阻塞或 systemd 抑制锁，需配合 --reboot）
        #[arg(short, long)]
        force: bool,
    },

    /// 立即重启当前电脑
    Reboot {
        /// 强制重启（忽略未保存程序阻塞或 systemd 抑制锁）
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Serialize)]
struct CliResponse<T: Serialize> {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_mode = cli.json;

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if json_mode {
                let payload: CliResponse<()> = CliResponse {
                    ok: false,
                    message: err.to_string(),
                    data: None,
                };
                println!(
                    "{}",
                    serde_json::to_string(&payload).unwrap_or_else(|_| {
                        "{\"ok\":false,\"message\":\"serialization error\"}".to_string()
                    })
                );
            } else {
                eprintln!("错误: {err}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), BootError> {
    let backend = create_backend(cli.mock);

    match cli.command {
        Commands::Status => {
            let state = backend.read_state()?;
            if cli.json {
                print_json("ok", Some(&state));
            } else {
                print_status_human(&state);
            }
        }
        Commands::List { all } => {
            let state = backend.read_state()?;
            if state.firmware != FirmwareType::Uefi {
                if cli.json {
                    print_json("检测到非 UEFI 固件环境", Some(&state));
                } else {
                    print_legacy_notice(&state);
                }
                return Ok(());
            }

            let filtered: Vec<&BootEntry> = state
                .entries
                .iter()
                .filter(|e| all || e.is_primary_os || e.is_current || e.is_default || e.is_next)
                .collect();

            if cli.json {
                print_json("ok", Some(&filtered));
            } else {
                print_entries_table(&filtered, all, state.entries.len());
            }
        }
        Commands::Next {
            target,
            reboot,
            force,
        } => {
            let state = backend.read_state()?;
            ensure_uefi(&state)?;
            let id = resolve_target_id(&state, &target)?;
            let target_name = find_name(&state, id);

            backend.set_boot_next(id)?;

            let msg = if reboot {
                backend.reboot(RebootOptions { force })?;
                format!(
                    "已设置下一次重启进入 {} (Boot{})，正在{}重启...",
                    target_name,
                    format_boot_id(id),
                    if force { "强制" } else { "" }
                )
            } else {
                format!(
                    "已设置下一次重启进入 {} (Boot{})，未立即重启。",
                    target_name,
                    format_boot_id(id)
                )
            };

            let updated = backend.read_state().ok();
            if cli.json {
                print_json(&msg, updated.as_ref());
            } else {
                println!("{msg}");
            }
        }
        Commands::ClearNext => {
            let state = backend.read_state()?;
            ensure_uefi(&state)?;
            backend.clear_boot_next()?;
            let msg = "已清除单次启动指定 (BootNext)，下次重启将按默认顺序引导。";
            let updated = backend.read_state().ok();
            if cli.json {
                print_json(msg, updated.as_ref());
            } else {
                println!("{msg}");
            }
        }
        Commands::Default {
            target,
            reboot,
            force,
        } => {
            let state = backend.read_state()?;
            ensure_uefi(&state)?;
            let id = resolve_target_id(&state, &target)?;
            let target_name = find_name(&state, id);

            backend.set_default_boot(id)?;

            let msg = if reboot {
                backend.reboot(RebootOptions { force })?;
                format!(
                    "已将 {} (Boot{}) 设为长期默认启动项，正在{}重启...",
                    target_name,
                    format_boot_id(id),
                    if force { "强制" } else { "" }
                )
            } else {
                format!(
                    "已将 {} (Boot{}) 设为长期默认启动项（未立即重启）。",
                    target_name,
                    format_boot_id(id)
                )
            };

            let updated = backend.read_state().ok();
            if cli.json {
                print_json(&msg, updated.as_ref());
            } else {
                println!("{msg}");
            }
        }
        Commands::Reboot { force } => {
            backend.reboot(RebootOptions { force })?;
            let msg = if force {
                "正在执行强制重启..."
            } else {
                "正在执行系统重启..."
            };
            if cli.json {
                print_json::<()>(msg, None);
            } else {
                println!("{msg}");
            }
        }
    }

    Ok(())
}

fn ensure_uefi(state: &BootState) -> Result<(), BootError> {
    if state.firmware != FirmwareType::Uefi {
        return Err(BootError::UnsupportedFirmware(state.firmware));
    }
    Ok(())
}

fn find_name(state: &BootState, id: u16) -> String {
    state
        .entries
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.display_name.clone())
        .unwrap_or_else(|| format!("Boot{}", format_boot_id(id)))
}

fn print_json<T: Serialize>(message: &str, data: Option<&T>) {
    let resp = CliResponse {
        ok: true,
        message: message.to_string(),
        data,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&resp).expect("serialize response")
    );
}

fn print_legacy_notice(state: &BootState) {
    println!("固件模式: {:?}", state.firmware);
    println!("提示: 当前电脑不是 UEFI 引导模式（可能为传统 Legacy BIOS / MBR）。");
    println!("NextBoot v1 通过主板 UEFI NVRAM 进行跨系统安全切换，暂不支持 Legacy BIOS。");
}

fn print_status_human(state: &BootState) {
    if state.is_mock {
        println!("[模拟模式 MOCK]");
    }
    println!("主机系统: {}", state.host_os);
    println!("固件模式: {:?}", state.firmware);
    if state.firmware != FirmwareType::Uefi {
        print_legacy_notice(state);
        return;
    }

    let resolve_label = |hex: &Option<String>| -> String {
        match hex {
            Some(id) => {
                let name = state
                    .entries
                    .iter()
                    .find(|e| &e.id_hex == id)
                    .map(|e| e.display_name.as_str())
                    .unwrap_or("未知");
                format!("{name} (Boot{id})")
            }
            None => "未设置".to_string(),
        }
    };

    println!("当前运行: {}", resolve_label(&state.boot_current));
    println!("长期默认: {}", resolve_label(&state.default_boot));
    println!("下次指定: {}", resolve_label(&state.boot_next));
    println!(
        "引导顺序: {}",
        state
            .boot_order
            .iter()
            .map(|id| format!("Boot{id}"))
            .collect::<Vec<_>>()
            .join(" -> ")
    );
}

fn print_entries_table(entries: &[&BootEntry], show_all: bool, total_count: usize) {
    println!(
        "{:<10} {:<24} {:<18} {}",
        "编号", "名称", "标记", "EFI 路径"
    );
    println!("{}", "-".repeat(78));
    for e in entries {
        let mut tags = Vec::new();
        if e.is_current {
            tags.push("当前");
        }
        if e.is_default {
            tags.push("默认");
        }
        if e.is_next {
            tags.push("下次");
        }
        if !e.is_primary_os {
            tags.push("高级");
        }
        let tag_str = if tags.is_empty() {
            "-".to_string()
        } else {
            tags.join(",")
        };
        println!(
            "Boot{:<6} {:<24} {:<18} {}",
            e.id_hex,
            e.display_name,
            tag_str,
            e.efi_path.as_deref().unwrap_or("-")
        );
    }

    if !show_all && total_count > entries.len() {
        println!(
            "\n已隐藏 {} 个非操作系统项（PXE/U 盘/固件等），使用 --all 查看全部。",
            total_count - entries.len()
        );
    }
}

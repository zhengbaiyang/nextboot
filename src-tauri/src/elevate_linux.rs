//! Linux privilege elevation helper via `pkexec` + `nextboot-cli`.

use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct CliJsonResponse {
    ok: bool,
    message: String,
}

fn find_cli_binary() -> Result<PathBuf, String> {
    // 1. Sibling binary next to the GUI executable (works in dev builds & portable tarballs)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("nextboot-cli");
            if sibling.is_file() {
                return Ok(sibling);
            }
        }
    }

    // 2. Standard system installation path from .deb package
    let usr_bin = PathBuf::from("/usr/bin/nextboot-cli");
    if usr_bin.is_file() {
        return Ok(usr_bin);
    }

    let usr_local_bin = PathBuf::from("/usr/local/bin/nextboot-cli");
    if usr_local_bin.is_file() {
        return Ok(usr_local_bin);
    }

    // 3. Fallback to PATH lookup via `which`
    if let Ok(output) = Command::new("which").arg("nextboot-cli").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Ok(PathBuf::from(path_str));
            }
        }
    }

    Err("未找到提权辅助程序 nextboot-cli（请确认已通过 .deb 安装或与主程序位于同一目录）。".to_string())
}

/// Execute `nextboot-cli` with `pkexec` so Ubuntu GUI stays unprivileged (X11 & Wayland safe)
/// and only prompts for Polkit authentication once per write/reboot action.
pub fn run_privileged_cli(args: &[&str]) -> Result<String, String> {
    let cli_path = find_cli_binary()?;

    let mut cmd = Command::new("pkexec");
    cmd.arg(&cli_path);
    cmd.arg("--json");
    cmd.args(args);

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "系统未安装 pkexec（PolicyKit），无法弹出图形提权窗口。请安装 pkexec 或使用 sudo nextboot-cli。".to_string()
        } else {
            format!("调用 pkexec 失败: {e}")
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if let Ok(parsed) = serde_json::from_str::<CliJsonResponse>(&stdout) {
        if parsed.ok {
            return Ok(parsed.message);
        }
        return Err(parsed.message);
    }

    if output.status.success() {
        return Ok(if stdout.is_empty() {
            "操作已完成".to_string()
        } else {
            stdout
        });
    }

    // pkexec exit code 126 = user dismissed dialog; 127 = auth failed / not authorized
    match output.status.code() {
        Some(126) => Err("已取消管理员授权。".to_string()),
        Some(127) => Err("管理员授权失败，请重试。".to_string()),
        _ => {
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("退出码 {:?}", output.status.code())
            };
            Err(format!("提权执行失败: {detail}"))
        }
    }
}

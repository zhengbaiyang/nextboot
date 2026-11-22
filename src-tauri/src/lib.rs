#[cfg(target_os = "linux")]
mod elevate_linux;
#[cfg(target_os = "windows")]
mod elevate_windows;

use boot_core::{create_backend, format_boot_id, parse_boot_id, BootState, RebootOptions};
use serde::{Deserialize, Serialize};
use tauri::State;

pub struct AppConfig {
    pub use_mock: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchMode {
    Next,
    Default,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    pub message: String,
    pub state: Option<BootState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl From<boot_core::BootError> for ApiError {
    fn from(err: boot_core::BootError) -> Self {
        let code = match &err {
            boot_core::BootError::PermissionDenied(_) => "permission_denied",
            boot_core::BootError::UnsupportedFirmware(_) => "unsupported_firmware",
            boot_core::BootError::InvalidBootId(_) => "invalid_boot_id",
            boot_core::BootError::EntryNotFound(_) => "entry_not_found",
            _ => "system_error",
        };
        Self {
            code: code.to_string(),
            message: err.to_string(),
        }
    }
}

#[tauri::command]
fn get_boot_state(config: State<'_, AppConfig>) -> Result<BootState, ApiError> {
    let backend = create_backend(config.use_mock);
    backend.read_state().map_err(ApiError::from)
}

#[tauri::command]
fn apply_boot_switch(
    config: State<'_, AppConfig>,
    target_id: String,
    mode: SwitchMode,
    reboot: bool,
    force_reboot: bool,
) -> Result<CommandResult, ApiError> {
    let parsed_id = parse_boot_id(&target_id).map_err(ApiError::from)?;
    let hex_id = format_boot_id(parsed_id);
    let backend = create_backend(config.use_mock);

    // On Linux non-mock & non-root, delegate mutation + optional reboot to pkexec nextboot-cli
    // so only a single Polkit password prompt is shown.
    #[cfg(target_os = "linux")]
    if !config.use_mock && !boot_core::env_requests_mock() && !boot_core::linux::LinuxBootBackend::is_root() {
        let subcmd = match mode {
            SwitchMode::Next => "next",
            SwitchMode::Default => "default",
        };
        let mut args: Vec<&str> = vec![subcmd, &hex_id];
        if reboot {
            args.push("--reboot");
            if force_reboot {
                args.push("--force");
            }
        }
        let message = elevate_linux::run_privileged_cli(&args).map_err(|msg| ApiError {
            code: "elevation_error".to_string(),
            message: msg,
        })?;
        let state = backend.read_state().ok();
        return Ok(CommandResult { message, state });
    }

    let message = match mode {
        SwitchMode::Next => {
            backend.set_boot_next(parsed_id).map_err(ApiError::from)?;
            if reboot {
                backend
                    .reboot(RebootOptions {
                        force: force_reboot,
                    })
                    .map_err(ApiError::from)?;
                format!("已设置下一次重启进入 Boot{hex_id}，正在重启...")
            } else {
                format!("已设置下一次重启进入 Boot{hex_id}（仅下次生效）。")
            }
        }
        SwitchMode::Default => {
            backend
                .set_default_boot(parsed_id)
                .map_err(ApiError::from)?;
            if reboot {
                backend
                    .reboot(RebootOptions {
                        force: force_reboot,
                    })
                    .map_err(ApiError::from)?;
                format!("已将 Boot{hex_id} 设为长期默认启动项，正在重启...")
            } else {
                format!("已将 Boot{hex_id} 设为长期默认启动项。")
            }
        }
    };

    let state = backend.read_state().ok();
    Ok(CommandResult { message, state })
}

#[tauri::command]
fn clear_boot_next(config: State<'_, AppConfig>) -> Result<CommandResult, ApiError> {
    let backend = create_backend(config.use_mock);

    #[cfg(target_os = "linux")]
    if !config.use_mock && !boot_core::env_requests_mock() && !boot_core::linux::LinuxBootBackend::is_root() {
        let message =
            elevate_linux::run_privileged_cli(&["clear-next"]).map_err(|msg| ApiError {
                code: "elevation_error".to_string(),
                message: msg,
            })?;
        let state = backend.read_state().ok();
        return Ok(CommandResult { message, state });
    }

    backend.clear_boot_next().map_err(ApiError::from)?;
    let state = backend.read_state().ok();
    Ok(CommandResult {
        message: "已清除下一次重启指定 (BootNext)，将恢复按默认顺序引导。".to_string(),
        state,
    })
}

#[tauri::command]
fn reboot_now(config: State<'_, AppConfig>, force: bool) -> Result<CommandResult, ApiError> {
    let backend = create_backend(config.use_mock);

    #[cfg(target_os = "linux")]
    if !config.use_mock && !boot_core::env_requests_mock() && !boot_core::linux::LinuxBootBackend::is_root() {
        let args: Vec<&str> = if force {
            vec!["reboot", "--force"]
        } else {
            vec!["reboot"]
        };
        let message = elevate_linux::run_privileged_cli(&args).map_err(|msg| ApiError {
            code: "elevation_error".to_string(),
            message: msg,
        })?;
        return Ok(CommandResult {
            message,
            state: None,
        });
    }

    backend
        .reboot(RebootOptions { force })
        .map_err(ApiError::from)?;
    let state = if config.use_mock || boot_core::env_requests_mock() {
        backend.read_state().ok()
    } else {
        None
    };

    Ok(CommandResult {
        message: if force {
            "正在执行强制重启...".to_string()
        } else {
            "正在重启系统...".to_string()
        },
        state,
    })
}

#[tauri::command]
fn relaunch_elevated(app: tauri::AppHandle) -> Result<(), ApiError> {
    #[cfg(target_os = "windows")]
    {
        elevate_windows::relaunch_as_admin().map_err(|msg| ApiError {
            code: "elevation_error".to_string(),
            message: msg,
        })?;
        app.exit(0);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err(ApiError {
            code: "unsupported".to_string(),
            message: "Linux 端会在应用修改时通过 pkexec 自动请求授权，无需重启本程序。".to_string(),
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let use_mock = std::env::args().any(|arg| arg == "--mock") || boot_core::env_requests_mock();

    tauri::Builder::default()
        .manage(AppConfig { use_mock })
        .invoke_handler(tauri::generate_handler![
            get_boot_state,
            apply_boot_switch,
            clear_boot_next,
            reboot_now,
            relaunch_elevated,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NextBoot desktop application");
}

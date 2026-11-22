//! Cross-platform UEFI NVRAM boot management engine for NextBoot.

pub mod classifier;
pub mod mock;
pub mod parser;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

pub use classifier::OsKind;
pub use parser::{format_boot_id, parse_boot_id};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Detected system firmware type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareType {
    Uefi,
    LegacyBios,
    Unknown,
}

/// Reboot behavior configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebootOptions {
    /// When true, bypass blocking apps (Windows) or systemd inhibitor locks (Linux).
    pub force: bool,
}

/// Structured representation of a single UEFI boot entry (`BootXXXX`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootEntry {
    pub id: u16,
    pub id_hex: String,
    pub description: String,
    pub display_name: String,
    pub efi_path: Option<String>,
    pub os_kind: OsKind,
    pub active: bool,
    pub is_primary_os: bool,
    pub is_current: bool,
    pub is_default: bool,
    pub is_next: bool,
    pub order_index: Option<usize>,
    pub risk_warning: Option<String>,
}

/// Complete snapshot of the system's UEFI boot state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootState {
    pub firmware: FirmwareType,
    pub host_os: String,
    pub is_mock: bool,
    pub is_elevated: bool,
    pub boot_current: Option<String>,
    pub boot_next: Option<String>,
    pub default_boot: Option<String>,
    pub boot_order: Vec<String>,
    pub entries: Vec<BootEntry>,
}

/// Errors produced by UEFI reading, writing, or rebooting operations.
#[derive(Debug, Error)]
pub enum BootError {
    #[error("无效的启动项编号: {0}（示例格式: 0001 或 Boot0001）")]
    InvalidBootId(String),

    #[error("未找到对应的 UEFI 启动项: Boot{0}")]
    EntryNotFound(String),

    #[error("当前主板固件模式 ({0:?}) 不支持 UEFI NVRAM 切换")]
    UnsupportedFirmware(FirmwareType),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("UEFI 数据解析失败: {0}")]
    ParseError(String),

    #[error("I/O 错误: {0}")]
    Io(String),

    #[error("系统调用失败: {0}")]
    SystemError(String),
}

/// Platform-agnostic interface for reading and updating UEFI boot targets.
pub trait BootBackend: Send + Sync {
    fn detect_firmware(&self) -> FirmwareType;
    fn read_state(&self) -> Result<BootState, BootError>;
    fn set_boot_next(&self, id: u16) -> Result<(), BootError>;
    fn clear_boot_next(&self) -> Result<(), BootError>;
    fn set_default_boot(&self, id: u16) -> Result<(), BootError>;
    fn reboot(&self, options: RebootOptions) -> Result<(), BootError>;
}

/// Return a human-readable host OS identifier.
pub fn current_host_os() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        "other"
    }
}

/// Check if mock mode is requested via environment variable `NEXTBOOT_MOCK=1`.
pub fn env_requests_mock() -> bool {
    std::env::var("NEXTBOOT_MOCK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Instantiate either the mock backend or the native OS UEFI backend.
pub fn create_backend(use_mock: bool) -> Box<dyn BootBackend> {
    if use_mock || env_requests_mock() {
        return Box::new(mock::MockBootBackend::new());
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsBootBackend::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBootBackend::new())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Box::new(mock::MockBootBackend::new())
    }
}

/// Resolve a user-provided target selector: either a hex boot ID (`0002`, `Boot0002`)
/// or a case-insensitive unique match against `display_name` / `description` (e.g. `windows`, `ubuntu`).
pub fn resolve_target_id(state: &BootState, selector: &str) -> Result<u16, BootError> {
    if let Ok(id) = parse_boot_id(selector) {
        if state.entries.iter().any(|e| e.id == id) {
            return Ok(id);
        }
    }

    let needle = selector.trim().to_lowercase();
    if needle.is_empty() {
        return Err(BootError::InvalidBootId(selector.to_string()));
    }

    // Prefer exact match on display_name or description among primary OS entries first
    let exact: Vec<&BootEntry> = state
        .entries
        .iter()
        .filter(|e| {
            e.display_name.to_lowercase() == needle || e.description.to_lowercase() == needle
        })
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0].id);
    }

    // Substring match among primary OS entries
    let primary_matches: Vec<&BootEntry> = state
        .entries
        .iter()
        .filter(|e| {
            e.is_primary_os
                && (e.display_name.to_lowercase().contains(&needle)
                    || e.description.to_lowercase().contains(&needle))
        })
        .collect();
    if primary_matches.len() == 1 {
        return Ok(primary_matches[0].id);
    }

    // Fallback substring match across all entries
    let all_matches: Vec<&BootEntry> = state
        .entries
        .iter()
        .filter(|e| {
            e.display_name.to_lowercase().contains(&needle)
                || e.description.to_lowercase().contains(&needle)
        })
        .collect();
    if all_matches.len() == 1 {
        return Ok(all_matches[0].id);
    }

    Err(BootError::EntryNotFound(selector.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_mock_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nextboot-test-{}-{}.json", std::process::id(), name))
    }

    #[test]
    fn mock_backend_next_and_default_flow() {
        let file = temp_mock_file("flow");
        let _ = std::fs::remove_file(&file);
        let backend = mock::MockBootBackend::with_state_file(file.clone());

        let initial = backend.read_state().unwrap();
        assert_eq!(initial.firmware, FirmwareType::Uefi);
        assert_eq!(initial.default_boot.as_deref(), Some("0001"));
        assert_eq!(initial.boot_next, None);

        // Resolve by name
        let win_id = resolve_target_id(&initial, "windows").unwrap();
        assert_eq!(win_id, 0x0002);

        // Set BootNext to Windows
        backend.set_boot_next(win_id).unwrap();
        let after_next = backend.read_state().unwrap();
        assert_eq!(after_next.boot_next.as_deref(), Some("0002"));

        // Clear BootNext
        backend.clear_boot_next().unwrap();
        let after_clear = backend.read_state().unwrap();
        assert_eq!(after_clear.boot_next, None);

        // Set default boot to Windows
        backend.set_default_boot(win_id).unwrap();
        let after_default = backend.read_state().unwrap();
        assert_eq!(after_default.default_boot.as_deref(), Some("0002"));
        assert_eq!(after_default.boot_order[0], "0002");

        let _ = std::fs::remove_file(&file);
    }
}

//! Simulated UEFI NVRAM backend for safe testing & UI development.

use crate::classifier::OsKind;
use crate::parser::format_boot_id;
use crate::{BootBackend, BootEntry, BootError, BootState, FirmwareType, RebootOptions};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockState {
    firmware: FirmwareType,
    boot_current: Option<u16>,
    boot_next: Option<u16>,
    boot_order: Vec<u16>,
    entries: Vec<MockRawEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockRawEntry {
    id: u16,
    description: String,
    display_name: String,
    efi_path: Option<String>,
    os_kind: OsKind,
    active: bool,
    is_primary_os: bool,
    risk_warning: Option<String>,
}

/// Stateful mock backend stored in a temporary JSON file so repeated CLI calls see mutations.
#[derive(Debug, Clone)]
pub struct MockBootBackend {
    state_file: PathBuf,
}

impl Default for MockBootBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBootBackend {
    pub fn new() -> Self {
        let state_file = std::env::var_os("NEXTBOOT_MOCK_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("nextboot-mock-state.json"));
        Self { state_file }
    }

    pub fn with_state_file(state_file: PathBuf) -> Self {
        Self { state_file }
    }

    fn default_mock_state() -> MockState {
        let is_legacy = std::env::var("NEXTBOOT_MOCK_LEGACY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if is_legacy {
            return MockState {
                firmware: FirmwareType::LegacyBios,
                boot_current: None,
                boot_next: None,
                boot_order: vec![],
                entries: vec![],
            };
        }

        // Default realistic dual-boot scenario matching the user's setup:
        // Ubuntu is primary (BootOrder[0]), Windows Boot Manager is secondary, currently booted in Windows or Ubuntu.
        let boot_current = if cfg!(target_os = "windows") {
            Some(0x0002)
        } else {
            Some(0x0001)
        };

        MockState {
            firmware: FirmwareType::Uefi,
            boot_current,
            boot_next: None,
            boot_order: vec![0x0001, 0x0002, 0x0003, 0x0004],
            entries: vec![
                MockRawEntry {
                    id: 0x0001,
                    description: "ubuntu".to_string(),
                    display_name: "Ubuntu".to_string(),
                    efi_path: Some("\\EFI\\ubuntu\\shimx64.efi".to_string()),
                    os_kind: OsKind::Ubuntu,
                    active: true,
                    is_primary_os: true,
                    risk_warning: None,
                },
                MockRawEntry {
                    id: 0x0002,
                    description: "Windows Boot Manager".to_string(),
                    display_name: "Windows".to_string(),
                    efi_path: Some("\\EFI\\Microsoft\\Boot\\bootmgfw.efi".to_string()),
                    os_kind: OsKind::Windows,
                    active: true,
                    is_primary_os: true,
                    risk_warning: None,
                },
                MockRawEntry {
                    id: 0x0003,
                    description: "UEFI: PXE IPv4 Realtek PCIe GBE".to_string(),
                    display_name: "UEFI: PXE IPv4 Realtek PCIe GBE".to_string(),
                    efi_path: None,
                    os_kind: OsKind::Network,
                    active: true,
                    is_primary_os: false,
                    risk_warning: Some(
                        "这是网络启动项 (PXE/HTTP)，远控选择可能导致开机卡死失联。".to_string(),
                    ),
                },
                MockRawEntry {
                    id: 0x0004,
                    description: "UEFI: Built-in EFI Shell".to_string(),
                    display_name: "UEFI: Built-in EFI Shell".to_string(),
                    efi_path: None,
                    os_kind: OsKind::Firmware,
                    active: true,
                    is_primary_os: false,
                    risk_warning: Some(
                        "这是主板固件或 UEFI Shell 工具，不会进入操作系统。".to_string(),
                    ),
                },
            ],
        }
    }

    fn load_state(&self) -> MockState {
        if let Ok(content) = fs::read_to_string(&self.state_file) {
            if let Ok(parsed) = serde_json::from_str::<MockState>(&content) {
                return parsed;
            }
        }
        Self::default_mock_state()
    }

    fn save_state(&self, state: &MockState) -> Result<(), BootError> {
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| BootError::Io(format!("failed to serialize mock state: {e}")))?;
        fs::write(&self.state_file, json)
            .map_err(|e| BootError::Io(format!("failed to write mock state: {e}")))
    }
}

impl BootBackend for MockBootBackend {
    fn detect_firmware(&self) -> FirmwareType {
        self.load_state().firmware
    }

    fn read_state(&self) -> Result<BootState, BootError> {
        let state = self.load_state();
        if state.firmware != FirmwareType::Uefi {
            return Ok(BootState {
                firmware: state.firmware,
                host_os: crate::current_host_os().to_string(),
                is_mock: true,
                is_elevated: true,
                boot_current: None,
                boot_next: None,
                default_boot: None,
                boot_order: vec![],
                entries: vec![],
            });
        }

        let default_boot = state.boot_order.first().copied();
        let mut entries: Vec<BootEntry> = state
            .entries
            .iter()
            .map(|raw| {
                let order_index = state.boot_order.iter().position(|&id| id == raw.id);
                BootEntry {
                    id: raw.id,
                    id_hex: format_boot_id(raw.id),
                    description: raw.description.clone(),
                    display_name: raw.display_name.clone(),
                    efi_path: raw.efi_path.clone(),
                    os_kind: raw.os_kind,
                    active: raw.active,
                    is_primary_os: raw.is_primary_os,
                    is_current: state.boot_current == Some(raw.id),
                    is_default: default_boot == Some(raw.id),
                    is_next: state.boot_next == Some(raw.id),
                    order_index,
                    risk_warning: raw.risk_warning.clone(),
                }
            })
            .collect();

        entries.sort_by_key(|e| (e.order_index.unwrap_or(usize::MAX), e.id));

        Ok(BootState {
            firmware: FirmwareType::Uefi,
            host_os: crate::current_host_os().to_string(),
            is_mock: true,
            is_elevated: true,
            boot_current: state.boot_current.map(format_boot_id),
            boot_next: state.boot_next.map(format_boot_id),
            default_boot: default_boot.map(format_boot_id),
            boot_order: state.boot_order.iter().copied().map(format_boot_id).collect(),
            entries,
        })
    }

    fn set_boot_next(&self, id: u16) -> Result<(), BootError> {
        let mut state = self.load_state();
        if state.firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(state.firmware));
        }
        if !state.entries.iter().any(|e| e.id == id) {
            return Err(BootError::EntryNotFound(format_boot_id(id)));
        }
        state.boot_next = Some(id);
        self.save_state(&state)
    }

    fn clear_boot_next(&self) -> Result<(), BootError> {
        let mut state = self.load_state();
        if state.firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(state.firmware));
        }
        state.boot_next = None;
        self.save_state(&state)
    }

    fn set_default_boot(&self, id: u16) -> Result<(), BootError> {
        let mut state = self.load_state();
        if state.firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(state.firmware));
        }
        if !state.entries.iter().any(|e| e.id == id) {
            return Err(BootError::EntryNotFound(format_boot_id(id)));
        }
        state.boot_order.retain(|&existing| existing != id);
        state.boot_order.insert(0, id);
        self.save_state(&state)
    }

    fn reboot(&self, _options: RebootOptions) -> Result<(), BootError> {
        // In mock mode we simulate consuming BootNext without restarting the host!
        let mut state = self.load_state();
        let target = state
            .boot_next
            .take()
            .or_else(|| state.boot_order.first().copied());
        if let Some(next_id) = target {
            state.boot_current = Some(next_id);
        }
        self.save_state(&state)
    }
}

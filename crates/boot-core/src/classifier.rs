//! Smart classification and safety filtering for UEFI boot entries.

use crate::parser::ParsedLoadOption;
use serde::{Deserialize, Serialize};

/// Identified operating system or device category for a UEFI boot entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsKind {
    Windows,
    Ubuntu,
    Debian,
    Fedora,
    Arch,
    Linux,
    MacOs,
    Network,
    Usb,
    Optical,
    Firmware,
    Unknown,
}

/// Classification result attached to a `BootEntry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryClassification {
    pub os_kind: OsKind,
    pub display_name: String,
    pub is_primary_os: bool,
    pub risk_warning: Option<String>,
}

/// Classify a parsed UEFI load option into an OS/device kind, friendly name, and safety metadata.
pub fn classify_load_option(parsed: &ParsedLoadOption) -> EntryClassification {
    let desc = parsed.description.trim();
    let desc_lower = desc.to_lowercase();
    let path_lower = parsed
        .device_path
        .file_path
        .as_deref()
        .unwrap_or("")
        .to_lowercase()
        .replace('/', "\\");

    // 1. Check explicit non-OS device paths or firmware apps first (unless overridden by clear OS EFI file)
    let is_windows_efi = path_lower.contains("\\efi\\microsoft\\")
        || path_lower.ends_with("bootmgfw.efi")
        || desc_lower.contains("windows boot manager")
        || desc_lower == "windows";

    let is_ubuntu_efi = path_lower.contains("\\efi\\ubuntu\\")
        || desc_lower == "ubuntu"
        || desc_lower.starts_with("ubuntu ");

    let is_debian_efi =
        path_lower.contains("\\efi\\debian\\") || desc_lower.contains("debian");
    let is_fedora_efi =
        path_lower.contains("\\efi\\fedora\\") || desc_lower.contains("fedora");
    let is_arch_efi = path_lower.contains("\\efi\\arch\\")
        || path_lower.contains("\\efi\\manjaro\\")
        || desc_lower.contains("arch linux")
        || desc_lower.contains("manjaro");
    let is_macos_efi = path_lower.contains("\\system\\library\\coreservices\\boot.efi")
        || path_lower.contains("\\efi\\oc\\opencore.efi")
        || desc_lower.contains("macos")
        || desc_lower.contains("opencore");
    let is_generic_linux_efi = path_lower.contains("\\efi\\grub")
        || path_lower.contains("\\efi\\systemd\\")
        || path_lower.contains("\\efi\\opensuse")
        || path_lower.contains("\\efi\\pop")
        || path_lower.contains("\\efi\\kali")
        || path_lower.contains("\\efi\\centos")
        || path_lower.contains("\\efi\\rocky")
        || path_lower.contains("\\efi\\nixos")
        || path_lower.contains("\\efi\\deepin")
        || path_lower.contains("\\efi\\kylin")
        || path_lower.contains("\\efi\\uos")
        || desc_lower.contains("linux")
        || desc_lower.contains("grub")
        || desc_lower.contains("opensuse")
        || desc_lower.contains("pop!_os")
        || desc_lower.contains("kali")
        || desc_lower.contains("deepin")
        || desc_lower.contains("kylin")
        || desc_lower.contains("uos");

    // Detect non-OS / risky boot targets
    let is_network = parsed.device_path.has_network
        || desc_lower.contains("pxe")
        || desc_lower.contains("ipv4")
        || desc_lower.contains("ipv6")
        || desc_lower.contains("http")
        || desc_lower.contains("network")
        || desc_lower.contains("nic ");

    let is_usb = parsed.device_path.has_usb
        || desc_lower.contains("usb")
        || desc_lower.contains("flash disk")
        || desc_lower.contains("removable");

    let is_optical = parsed.device_path.has_cdrom
        || desc_lower.contains("cd/dvd")
        || desc_lower.contains("cdrom")
        || desc_lower.contains("dvd");

    let is_firmware_tool = parsed.is_category_app
        || parsed.device_path.has_firmware_volume
        || desc_lower.contains("efi shell")
        || desc_lower.contains("uefi shell")
        || desc_lower.contains("bios setup")
        || desc_lower.contains("firmware")
        || desc_lower.contains("diagnostic")
        || desc_lower.contains("setup utility");

    let (os_kind, default_label) = if is_windows_efi && !is_usb && !is_optical {
        (OsKind::Windows, "Windows")
    } else if is_ubuntu_efi && !is_usb && !is_optical {
        (OsKind::Ubuntu, "Ubuntu")
    } else if is_debian_efi && !is_usb && !is_optical {
        (OsKind::Debian, "Debian")
    } else if is_fedora_efi && !is_usb && !is_optical {
        (OsKind::Fedora, "Fedora")
    } else if is_arch_efi && !is_usb && !is_optical {
        (OsKind::Arch, "Arch Linux")
    } else if is_macos_efi && !is_usb && !is_optical {
        (OsKind::MacOs, "macOS")
    } else if is_generic_linux_efi && !is_usb && !is_optical {
        (OsKind::Linux, "Linux")
    } else if is_network {
        (OsKind::Network, "网络启动 (PXE)")
    } else if is_usb {
        (OsKind::Usb, "USB 外部设备")
    } else if is_optical {
        (OsKind::Optical, "光驱设备")
    } else if is_firmware_tool {
        (OsKind::Firmware, "主板固件 / EFI 工具")
    } else {
        (OsKind::Unknown, "其他 UEFI 启动项")
    };

    let is_primary_os = !parsed.hidden
        && parsed.active
        && matches!(
            os_kind,
            OsKind::Windows
                | OsKind::Ubuntu
                | OsKind::Debian
                | OsKind::Fedora
                | OsKind::Arch
                | OsKind::Linux
                | OsKind::MacOs
        );

    let display_name = match os_kind {
        OsKind::Windows => {
            if desc_lower == "windows boot manager" || desc.is_empty() {
                "Windows".to_string()
            } else {
                desc.to_string()
            }
        }
        OsKind::Ubuntu => {
            if desc_lower == "ubuntu" || desc.is_empty() {
                "Ubuntu".to_string()
            } else {
                desc.to_string()
            }
        }
        _ => {
            if desc.is_empty() {
                default_label.to_string()
            } else {
                desc.to_string()
            }
        }
    };

    let risk_warning = match os_kind {
        OsKind::Network => Some("这是网络启动项 (PXE/HTTP)，远控选择可能导致开机卡死失联。".to_string()),
        OsKind::Usb => Some("这是外部 USB 设备启动项，拔出后可能导致无法正常进入系统。".to_string()),
        OsKind::Optical => Some("这是光驱启动项，通常不是本地操作系统。".to_string()),
        OsKind::Firmware => Some("这是主板固件或 UEFI Shell 工具，不会进入操作系统。".to_string()),
        OsKind::Unknown => Some("未能识别为常见操作系统，切换前请核对 EFI 路径以防远控失联。".to_string()),
        _ if !parsed.active => Some("该启动项在 UEFI 中处于未激活状态 (Inactive)。".to_string()),
        _ => None,
    };

    EntryClassification {
        os_kind,
        display_name,
        is_primary_os,
        risk_warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{DevicePathSummary, LOAD_OPTION_ACTIVE};

    #[test]
    fn classifies_windows_and_ubuntu_as_primary_os() {
        let win = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: "Windows Boot Manager".to_string(),
            device_path: DevicePathSummary {
                file_path: Some("\\EFI\\Microsoft\\Boot\\bootmgfw.efi".to_string()),
                has_hard_drive: true,
                ..Default::default()
            },
        };
        let win_cls = classify_load_option(&win);
        assert_eq!(win_cls.os_kind, OsKind::Windows);
        assert_eq!(win_cls.display_name, "Windows");
        assert!(win_cls.is_primary_os);
        assert!(win_cls.risk_warning.is_none());

        let ubu = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: "ubuntu".to_string(),
            device_path: DevicePathSummary {
                file_path: Some("\\EFI\\ubuntu\\shimx64.efi".to_string()),
                has_hard_drive: true,
                ..Default::default()
            },
        };
        let ubu_cls = classify_load_option(&ubu);
        assert_eq!(ubu_cls.os_kind, OsKind::Ubuntu);
        assert_eq!(ubu_cls.display_name, "Ubuntu");
        assert!(ubu_cls.is_primary_os);
        assert!(ubu_cls.risk_warning.is_none());
    }

    #[test]
    fn filters_out_pxe_and_usb_entries() {
        let pxe = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: "UEFI: PXE IPv4 Intel(R) Ethernet".to_string(),
            device_path: DevicePathSummary {
                has_network: true,
                ..Default::default()
            },
        };
        let cls = classify_load_option(&pxe);
        assert_eq!(cls.os_kind, OsKind::Network);
        assert!(!cls.is_primary_os);
        assert!(cls.risk_warning.is_some());
    }
}

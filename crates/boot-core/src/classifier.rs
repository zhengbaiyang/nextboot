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
    Manjaro,
    LinuxMint,
    PopOs,
    Opensuse,
    Kali,
    Redhat,
    Nixos,
    Gentoo,
    Deepin,
    Kylin,
    Endeavour,
    Zorin,
    Elementary,
    Steamos,
    Proxmox,
    Alpine,
    VoidLinux,
    Linux,
    MacOs,
    Network,
    Usb,
    Optical,
    Firmware,
    Unknown,
}

impl OsKind {
    /// Returns true if this category represents a recognized operating system.
    pub fn is_os(self) -> bool {
        !matches!(
            self,
            OsKind::Network | OsKind::Usb | OsKind::Optical | OsKind::Firmware | OsKind::Unknown
        )
    }
}

/// Classification result attached to a `BootEntry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryClassification {
    pub os_kind: OsKind,
    pub os_label: String,
    pub display_name: String,
    pub is_primary_os: bool,
    pub risk_warning: Option<String>,
}

/// Match a standalone word/token in a lowercased string (bounded by start/end or non-alphanumeric chars).
fn has_token(haystack: &str, token: &str) -> bool {
    let bytes = haystack.as_bytes();
    let tlen = token.len();
    if tlen == 0 || tlen > bytes.len() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(token) {
        let idx = start + pos;
        let before_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric();
        let after_idx = idx + tlen;
        let after_ok = after_idx == bytes.len() || !bytes[after_idx].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

/// Classify a parsed UEFI load option into an OS/device kind, friendly name, and safety metadata.
pub fn classify_load_option(parsed: &ParsedLoadOption) -> EntryClassification {
    let desc = parsed.description.trim();
    let desc_lower = desc.to_lowercase();
    let raw_path_lower = parsed
        .device_path
        .file_path
        .as_deref()
        .unwrap_or("")
        .to_lowercase()
        .replace('/', "\\");
    let path_lower = if !raw_path_lower.is_empty() && !raw_path_lower.starts_with('\\') {
        format!("\\{raw_path_lower}")
    } else {
        raw_path_lower
    };

    // Detect non-OS / risky boot targets first so removable/firmware entries are not misclassified
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

    let can_be_local_os =
        !is_usb && !is_optical && !parsed.is_category_app && !parsed.device_path.has_firmware_volume;

    // Specific OS detection (ordered so derivatives match before parent distros)
    let detected_os: Option<(OsKind, &'static str)> = if !can_be_local_os {
        None
    } else if path_lower.contains("\\efi\\microsoft\\")
        || path_lower.ends_with("bootmgfw.efi")
        || desc_lower.contains("windows boot manager")
        || desc_lower == "windows"
        || desc_lower.starts_with("windows ")
    {
        Some((OsKind::Windows, "Windows"))
    } else if path_lower.contains("\\system\\library\\coreservices\\boot.efi")
        || path_lower.contains("\\efi\\oc\\opencore.efi")
        || path_lower.contains("\\efi\\apple\\")
        || path_lower.contains("\\efi\\clover\\")
        || desc_lower.contains("macos")
        || desc_lower.contains("mac os")
        || desc_lower.contains("os x")
        || desc_lower.contains("opencore")
        || desc_lower.contains("clover")
    {
        Some((OsKind::MacOs, "macOS"))
    } else if path_lower.contains("\\efi\\linuxmint")
        || path_lower.contains("\\efi\\mint\\")
        || desc_lower.contains("linux mint")
        || desc_lower.contains("linuxmint")
    {
        Some((OsKind::LinuxMint, "Linux Mint"))
    } else if path_lower.contains("\\efi\\pop_os")
        || path_lower.contains("\\efi\\pop\\")
        || desc_lower.contains("pop!_os")
        || desc_lower.contains("pop_os")
        || desc_lower.contains("pop os")
    {
        Some((OsKind::PopOs, "Pop!_OS"))
    } else if path_lower.contains("\\efi\\zorin") || desc_lower.contains("zorin") {
        Some((OsKind::Zorin, "Zorin OS"))
    } else if path_lower.contains("\\efi\\elementary") || desc_lower.contains("elementary") {
        Some((OsKind::Elementary, "elementary OS"))
    } else if path_lower.contains("\\efi\\deepin")
        || path_lower.contains("\\efi\\uos")
        || path_lower.contains("\\efi\\uniontech")
        || desc_lower.contains("deepin")
        || desc_lower.contains("uniontech")
        || desc_lower.contains("统信")
        || has_token(&desc_lower, "uos")
    {
        Some((OsKind::Deepin, "Deepin / UOS"))
    } else if path_lower.contains("\\efi\\openkylin")
        || path_lower.contains("\\efi\\kylin")
        || path_lower.contains("\\efi\\ubuntukylin")
        || desc_lower.contains("openkylin")
        || desc_lower.contains("kylin")
        || desc_lower.contains("麒麟")
    {
        Some((OsKind::Kylin, "银河麒麟"))
    } else if path_lower.contains("\\efi\\kali") || desc_lower.contains("kali") {
        Some((OsKind::Kali, "Kali Linux"))
    } else if path_lower.contains("\\efi\\proxmox") || desc_lower.contains("proxmox") {
        Some((OsKind::Proxmox, "Proxmox VE"))
    } else if path_lower.contains("\\efi\\bazzite") || desc_lower.contains("bazzite") {
        Some((OsKind::Steamos, "Bazzite"))
    } else if path_lower.contains("\\efi\\steamos")
        || path_lower.contains("\\efi\\steam\\")
        || desc_lower.contains("steamos")
        || desc_lower.contains("steam os")
        || desc_lower.contains("holoiso")
    {
        Some((OsKind::Steamos, "SteamOS"))
    } else if path_lower.contains("\\efi\\manjaro") || desc_lower.contains("manjaro") {
        Some((OsKind::Manjaro, "Manjaro"))
    } else if path_lower.contains("\\efi\\cachyos") || desc_lower.contains("cachyos") {
        Some((OsKind::Endeavour, "CachyOS"))
    } else if path_lower.contains("\\efi\\garuda") || desc_lower.contains("garuda") {
        Some((OsKind::Endeavour, "Garuda"))
    } else if path_lower.contains("\\efi\\endeavour") || desc_lower.contains("endeavour") {
        Some((OsKind::Endeavour, "EndeavourOS"))
    } else if path_lower.contains("\\efi\\arch\\")
        || path_lower.contains("\\efi\\archlinux")
        || path_lower.contains("\\efi\\arch_")
        || path_lower.contains("\\efi\\arch-")
        || desc_lower.contains("arch linux")
        || desc_lower.contains("archlinux")
        || has_token(&desc_lower, "arch")
    {
        Some((OsKind::Arch, "Arch Linux"))
    } else if path_lower.contains("\\efi\\opensuse")
        || path_lower.contains("\\efi\\suse")
        || path_lower.contains("\\efi\\sles")
        || desc_lower.contains("opensuse")
        || desc_lower.contains("suse")
        || has_token(&desc_lower, "sles")
    {
        Some((OsKind::Opensuse, "openSUSE"))
    } else if path_lower.contains("\\efi\\rocky") || desc_lower.contains("rocky") {
        Some((OsKind::Redhat, "Rocky Linux"))
    } else if path_lower.contains("\\efi\\almalinux")
        || path_lower.contains("\\efi\\alma\\")
        || desc_lower.contains("almalinux")
        || desc_lower.contains("alma linux")
    {
        Some((OsKind::Redhat, "AlmaLinux"))
    } else if path_lower.contains("\\efi\\centos") || desc_lower.contains("centos") {
        Some((OsKind::Redhat, "CentOS"))
    } else if path_lower.contains("\\efi\\redhat")
        || path_lower.contains("\\efi\\rhel")
        || desc_lower.contains("red hat")
        || desc_lower.contains("redhat")
        || has_token(&desc_lower, "rhel")
    {
        Some((OsKind::Redhat, "Red Hat"))
    } else if path_lower.contains("\\efi\\nixos")
        || desc_lower.contains("nixos")
        || desc_lower.contains("nix os")
    {
        Some((OsKind::Nixos, "NixOS"))
    } else if path_lower.contains("\\efi\\gentoo") || desc_lower.contains("gentoo") {
        Some((OsKind::Gentoo, "Gentoo"))
    } else if path_lower.contains("\\efi\\alpine") || desc_lower.contains("alpine") {
        Some((OsKind::Alpine, "Alpine Linux"))
    } else if path_lower.contains("\\efi\\void")
        || desc_lower.contains("void linux")
        || desc_lower.contains("voidlinux")
        || has_token(&desc_lower, "void")
    {
        Some((OsKind::VoidLinux, "Void Linux"))
    } else if path_lower.contains("\\efi\\nobara")
        || desc_lower.contains("nobara")
        || path_lower.contains("\\efi\\mx")
        || desc_lower.contains("mx linux")
        || desc_lower.contains("mxlinux")
        || path_lower.contains("\\efi\\solus")
        || path_lower.contains("\\efi\\com.solus-project")
        || desc_lower.contains("solus")
        || path_lower.contains("\\efi\\slackware")
        || desc_lower.contains("slackware")
    {
        Some((OsKind::Linux, "Linux"))
    } else if path_lower.contains("\\efi\\ubuntu\\")
        || desc_lower == "ubuntu"
        || desc_lower.starts_with("ubuntu ")
        || desc_lower.contains("ubuntu")
        || desc_lower.contains("kubuntu")
        || desc_lower.contains("xubuntu")
        || desc_lower.contains("lubuntu")
    {
        Some((OsKind::Ubuntu, "Ubuntu"))
    } else if path_lower.contains("\\efi\\debian\\") || desc_lower.contains("debian") {
        Some((OsKind::Debian, "Debian"))
    } else if path_lower.contains("\\efi\\fedora\\") || desc_lower.contains("fedora") {
        Some((OsKind::Fedora, "Fedora"))
    } else if path_lower.contains("\\efi\\grub")
        || path_lower.contains("\\efi\\systemd\\")
        || path_lower.contains("\\efi\\refind")
        || path_lower.contains("\\efi\\linux")
        || path_lower.contains("\\efi\\limine")
        || desc_lower.contains("linux")
        || desc_lower.contains("grub")
        || desc_lower.contains("systemd-boot")
        || desc_lower.contains("refind")
    {
        Some((OsKind::Linux, "Linux"))
    } else {
        None
    };

    let (os_kind, os_label_str) = if let Some(detected) = detected_os {
        detected
    } else if is_network {
        (OsKind::Network, "网络启动 (PXE)")
    } else if is_usb {
        (OsKind::Usb, "USB 设备")
    } else if is_optical {
        (OsKind::Optical, "光驱设备")
    } else if is_firmware_tool {
        (OsKind::Firmware, "固件工具")
    } else {
        (OsKind::Unknown, "未知项")
    };

    let os_label = os_label_str.to_string();
    let is_primary_os = !parsed.hidden && parsed.active && os_kind.is_os();

    let display_name = if desc.is_empty() || desc_lower == os_label.to_lowercase() {
        os_label.clone()
    } else {
        match os_kind {
            OsKind::Windows if desc_lower == "windows boot manager" => "Windows".to_string(),
            OsKind::Arch if desc_lower == "arch" => "Arch Linux".to_string(),
            OsKind::Kali if desc_lower == "kali" => "Kali Linux".to_string(),
            OsKind::Alpine if desc_lower == "alpine" => "Alpine Linux".to_string(),
            OsKind::VoidLinux if desc_lower == "void" => "Void Linux".to_string(),
            OsKind::Proxmox if desc_lower == "proxmox" => "Proxmox VE".to_string(),
            OsKind::Deepin if desc_lower == "deepin" || desc_lower == "uos" => {
                "Deepin / UOS".to_string()
            }
            OsKind::Kylin if desc_lower == "kylin" => "银河麒麟".to_string(),
            OsKind::Zorin if desc_lower == "zorin" => "Zorin OS".to_string(),
            OsKind::Elementary if desc_lower == "elementary" => "elementary OS".to_string(),
            OsKind::PopOs if desc_lower == "pop_os" || desc_lower == "pop os" => {
                "Pop!_OS".to_string()
            }
            OsKind::Redhat if desc_lower == "rocky" => "Rocky Linux".to_string(),
            OsKind::Redhat if desc_lower == "redhat" || desc_lower == "rhel" => {
                "Red Hat".to_string()
            }
            _ => desc.to_string(),
        }
    };

    let risk_warning = match os_kind {
        OsKind::Network => {
            Some("这是网络启动项 (PXE/HTTP)，远控选择可能导致开机卡死失联。".to_string())
        }
        OsKind::Usb => {
            Some("这是外部 USB 设备启动项，拔出后可能导致无法正常进入系统。".to_string())
        }
        OsKind::Optical => Some("这是光驱启动项，通常不是本地操作系统。".to_string()),
        OsKind::Firmware => Some("这是主板固件或 UEFI Shell 工具，不会进入操作系统。".to_string()),
        OsKind::Unknown => {
            Some("未能识别为常见操作系统，切换前请核对 EFI 路径以防远控失联。".to_string())
        }
        _ if !parsed.active => {
            Some("该启动项在 UEFI 中处于未激活状态 (Inactive)。".to_string())
        }
        _ => None,
    };

    EntryClassification {
        os_kind,
        os_label,
        display_name,
        is_primary_os,
        risk_warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{DevicePathSummary, LOAD_OPTION_ACTIVE};

    fn make_hd_option(desc: &str, efi_path: &str) -> ParsedLoadOption {
        ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: desc.to_string(),
            device_path: DevicePathSummary {
                file_path: Some(efi_path.to_string()),
                has_hard_drive: true,
                ..Default::default()
            },
        }
    }

    #[test]
    fn classifies_windows_and_ubuntu_as_primary_os() {
        let win = make_hd_option("Windows Boot Manager", "\\EFI\\Microsoft\\Boot\\bootmgfw.efi");
        let win_cls = classify_load_option(&win);
        assert_eq!(win_cls.os_kind, OsKind::Windows);
        assert_eq!(win_cls.os_label, "Windows");
        assert_eq!(win_cls.display_name, "Windows");
        assert!(win_cls.is_primary_os);
        assert!(win_cls.risk_warning.is_none());

        let ubu = make_hd_option("ubuntu", "\\EFI\\ubuntu\\shimx64.efi");
        let ubu_cls = classify_load_option(&ubu);
        assert_eq!(ubu_cls.os_kind, OsKind::Ubuntu);
        assert_eq!(ubu_cls.os_label, "Ubuntu");
        assert_eq!(ubu_cls.display_name, "Ubuntu");
        assert!(ubu_cls.is_primary_os);
        assert!(ubu_cls.risk_warning.is_none());
    }

    #[test]
    fn classifies_major_linux_distributions() {
        let cases = [
            (
                "debian",
                "\\EFI\\debian\\shimx64.efi",
                OsKind::Debian,
                "Debian",
                "Debian",
            ),
            (
                "Fedora",
                "\\EFI\\fedora\\shimx64.efi",
                OsKind::Fedora,
                "Fedora",
                "Fedora",
            ),
            (
                "Arch Linux",
                "\\EFI\\arch\\grubx64.efi",
                OsKind::Arch,
                "Arch Linux",
                "Arch Linux",
            ),
            (
                "Manjaro",
                "\\EFI\\Manjaro\\grubx64.efi",
                OsKind::Manjaro,
                "Manjaro",
                "Manjaro",
            ),
            (
                "Linux Mint 22",
                "\\EFI\\ubuntu\\shimx64.efi",
                OsKind::LinuxMint,
                "Linux Mint",
                "Linux Mint 22",
            ),
            (
                "Pop!_OS 22.04 LTS",
                "\\EFI\\systemd\\systemd-bootx64.efi",
                OsKind::PopOs,
                "Pop!_OS",
                "Pop!_OS 22.04 LTS",
            ),
            (
                "opensuse-secureboot",
                "\\EFI\\opensuse\\shim.efi",
                OsKind::Opensuse,
                "openSUSE",
                "opensuse-secureboot",
            ),
            (
                "kali",
                "\\EFI\\kali\\grubx64.efi",
                OsKind::Kali,
                "Kali Linux",
                "Kali Linux",
            ),
            (
                "Rocky Linux 9",
                "\\EFI\\rocky\\shimx64.efi",
                OsKind::Redhat,
                "Rocky Linux",
                "Rocky Linux 9",
            ),
            (
                "AlmaLinux",
                "\\EFI\\almalinux\\shimx64.efi",
                OsKind::Redhat,
                "AlmaLinux",
                "AlmaLinux",
            ),
            (
                "CentOS Stream",
                "\\EFI\\centos\\shimx64.efi",
                OsKind::Redhat,
                "CentOS",
                "CentOS Stream",
            ),
            (
                "Red Hat Enterprise Linux",
                "\\EFI\\redhat\\shimx64.efi",
                OsKind::Redhat,
                "Red Hat",
                "Red Hat Enterprise Linux",
            ),
            (
                "NixOS",
                "\\EFI\\nixos\\grubx64.efi",
                OsKind::Nixos,
                "NixOS",
                "NixOS",
            ),
            (
                "gentoo",
                "\\EFI\\gentoo\\grubx64.efi",
                OsKind::Gentoo,
                "Gentoo",
                "Gentoo",
            ),
            (
                "deepin",
                "\\EFI\\deepin\\shimx64.efi",
                OsKind::Deepin,
                "Deepin / UOS",
                "Deepin / UOS",
            ),
            (
                "UOS 20",
                "\\EFI\\uos\\shimx64.efi",
                OsKind::Deepin,
                "Deepin / UOS",
                "UOS 20",
            ),
            (
                "Kylin V10",
                "\\EFI\\kylin\\grubx64.efi",
                OsKind::Kylin,
                "银河麒麟",
                "Kylin V10",
            ),
            (
                "openKylin 2.0",
                "\\EFI\\openkylin\\shimx64.efi",
                OsKind::Kylin,
                "银河麒麟",
                "openKylin 2.0",
            ),
            (
                "endeavouros",
                "\\EFI\\endeavouros\\grubx64.efi",
                OsKind::Endeavour,
                "EndeavourOS",
                "EndeavourOS",
            ),
            (
                "CachyOS",
                "\\EFI\\cachyos\\grubx64.efi",
                OsKind::Endeavour,
                "CachyOS",
                "CachyOS",
            ),
            (
                "Garuda Linux",
                "\\EFI\\garuda\\grubx64.efi",
                OsKind::Endeavour,
                "Garuda",
                "Garuda Linux",
            ),
            (
                "Zorin OS",
                "\\EFI\\zorin\\shimx64.efi",
                OsKind::Zorin,
                "Zorin OS",
                "Zorin OS",
            ),
            (
                "elementary OS",
                "\\EFI\\elementary\\shimx64.efi",
                OsKind::Elementary,
                "elementary OS",
                "elementary OS",
            ),
            (
                "SteamOS",
                "\\EFI\\steamos\\steamcl.efi",
                OsKind::Steamos,
                "SteamOS",
                "SteamOS",
            ),
            (
                "Bazzite",
                "\\EFI\\fedora\\shimx64.efi",
                OsKind::Steamos,
                "Bazzite",
                "Bazzite",
            ),
            (
                "proxmox",
                "\\EFI\\proxmox\\grubx64.efi",
                OsKind::Proxmox,
                "Proxmox VE",
                "Proxmox VE",
            ),
            (
                "Alpine Linux",
                "\\EFI\\alpine\\grubx64.efi",
                OsKind::Alpine,
                "Alpine Linux",
                "Alpine Linux",
            ),
            (
                "Void Linux",
                "\\EFI\\void_grub\\grubx64.efi",
                OsKind::VoidLinux,
                "Void Linux",
                "Void Linux",
            ),
        ];

        for (desc, path, expected_kind, expected_label, expected_display) in cases {
            let opt = make_hd_option(desc, path);
            let cls = classify_load_option(&opt);
            assert_eq!(cls.os_kind, expected_kind, "desc={desc}, path={path}");
            assert_eq!(cls.os_label, expected_label, "desc={desc}, path={path}");
            assert_eq!(
                cls.display_name, expected_display,
                "desc={desc}, path={path}"
            );
            assert!(cls.is_primary_os, "expected primary OS for {desc}");
            assert!(cls.risk_warning.is_none(), "unexpected warning for {desc}");
        }
    }

    #[test]
    fn classifies_generic_linux_bootloaders_and_other_distros() {
        let cases = [
            ("rEFInd Boot Manager", "\\EFI\\refind\\refind_x64.efi"),
            ("Linux Boot Manager", "\\EFI\\systemd\\systemd-bootx64.efi"),
            ("GRUB", "\\EFI\\grub\\grubx64.efi"),
            ("Solus", "\\EFI\\com.solus-project\\bootloaderx64.efi"),
            ("MX23", "\\EFI\\MX23\\grubx64.efi"),
            ("Nobara Linux", "\\EFI\\fedora\\shimx64.efi"),
            ("Slackware", "\\EFI\\Slackware\\elilo.efi"),
        ];

        for (desc, path) in cases {
            let opt = make_hd_option(desc, path);
            let cls = classify_load_option(&opt);
            assert_eq!(cls.os_kind, OsKind::Linux, "desc={desc}, path={path}");
            assert_eq!(cls.os_label, "Linux", "desc={desc}, path={path}");
            assert!(cls.is_primary_os, "expected primary OS for {desc}");
            assert!(cls.risk_warning.is_none());
        }
    }

    #[test]
    fn classifies_macos_entries() {
        let apple = make_hd_option("macOS", "\\System\\Library\\CoreServices\\boot.efi");
        let apple_cls = classify_load_option(&apple);
        assert_eq!(apple_cls.os_kind, OsKind::MacOs);
        assert_eq!(apple_cls.os_label, "macOS");
        assert!(apple_cls.is_primary_os);

        let oc = make_hd_option("OpenCore", "\\EFI\\OC\\OpenCore.efi");
        let oc_cls = classify_load_option(&oc);
        assert_eq!(oc_cls.os_kind, OsKind::MacOs);
        assert_eq!(oc_cls.os_label, "macOS");
        assert!(oc_cls.is_primary_os);
    }

    #[test]
    fn filters_out_pxe_usb_optical_firmware_and_unknown() {
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
        let pxe_cls = classify_load_option(&pxe);
        assert_eq!(pxe_cls.os_kind, OsKind::Network);
        assert_eq!(pxe_cls.os_label, "网络启动 (PXE)");
        assert!(!pxe_cls.is_primary_os);
        assert!(pxe_cls.risk_warning.is_some());

        // USB installer containing "Ubuntu" must still be classified as USB, not primary OS
        let usb = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: "UEFI: SanDisk USB Ubuntu Live".to_string(),
            device_path: DevicePathSummary {
                has_usb: true,
                file_path: Some("\\EFI\\BOOT\\BOOTX64.EFI".to_string()),
                ..Default::default()
            },
        };
        let usb_cls = classify_load_option(&usb);
        assert_eq!(usb_cls.os_kind, OsKind::Usb);
        assert_eq!(usb_cls.os_label, "USB 设备");
        assert!(!usb_cls.is_primary_os);
        assert!(usb_cls.risk_warning.is_some());

        let optical = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: "UEFI: SATA DVD-ROM".to_string(),
            device_path: DevicePathSummary {
                has_cdrom: true,
                ..Default::default()
            },
        };
        let opt_cls = classify_load_option(&optical);
        assert_eq!(opt_cls.os_kind, OsKind::Optical);
        assert_eq!(opt_cls.os_label, "光驱设备");
        assert!(!opt_cls.is_primary_os);
        assert!(opt_cls.risk_warning.is_some());

        let fw = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: true,
            description: "UEFI: Built-in EFI Shell".to_string(),
            device_path: DevicePathSummary {
                has_firmware_volume: true,
                ..Default::default()
            },
        };
        let fw_cls = classify_load_option(&fw);
        assert_eq!(fw_cls.os_kind, OsKind::Firmware);
        assert_eq!(fw_cls.os_label, "固件工具");
        assert!(!fw_cls.is_primary_os);
        assert!(fw_cls.risk_warning.is_some());

        let unknown = ParsedLoadOption {
            attributes: LOAD_OPTION_ACTIVE,
            active: true,
            hidden: false,
            is_category_app: false,
            description: "Custom Vendor Boot".to_string(),
            device_path: DevicePathSummary::default(),
        };
        let unk_cls = classify_load_option(&unknown);
        assert_eq!(unk_cls.os_kind, OsKind::Unknown);
        assert_eq!(unk_cls.os_label, "未知项");
        assert!(!unk_cls.is_primary_os);
        assert!(unk_cls.risk_warning.is_some());
    }

    #[test]
    fn os_kind_serializes_to_exact_contract_snake_case() {
        let expected = [
            (OsKind::Windows, "\"windows\""),
            (OsKind::Ubuntu, "\"ubuntu\""),
            (OsKind::Debian, "\"debian\""),
            (OsKind::Fedora, "\"fedora\""),
            (OsKind::Arch, "\"arch\""),
            (OsKind::Manjaro, "\"manjaro\""),
            (OsKind::LinuxMint, "\"linux_mint\""),
            (OsKind::PopOs, "\"pop_os\""),
            (OsKind::Opensuse, "\"opensuse\""),
            (OsKind::Kali, "\"kali\""),
            (OsKind::Redhat, "\"redhat\""),
            (OsKind::Nixos, "\"nixos\""),
            (OsKind::Gentoo, "\"gentoo\""),
            (OsKind::Deepin, "\"deepin\""),
            (OsKind::Kylin, "\"kylin\""),
            (OsKind::Endeavour, "\"endeavour\""),
            (OsKind::Zorin, "\"zorin\""),
            (OsKind::Elementary, "\"elementary\""),
            (OsKind::Steamos, "\"steamos\""),
            (OsKind::Proxmox, "\"proxmox\""),
            (OsKind::Alpine, "\"alpine\""),
            (OsKind::VoidLinux, "\"void_linux\""),
            (OsKind::Linux, "\"linux\""),
            (OsKind::MacOs, "\"mac_os\""),
            (OsKind::Network, "\"network\""),
            (OsKind::Usb, "\"usb\""),
            (OsKind::Optical, "\"optical\""),
            (OsKind::Firmware, "\"firmware\""),
            (OsKind::Unknown, "\"unknown\""),
        ];

        assert_eq!(expected.len(), 29);
        for (kind, json_str) in expected {
            assert_eq!(serde_json::to_string(&kind).unwrap(), json_str);
            let roundtrip: OsKind = serde_json::from_str(json_str).unwrap();
            assert_eq!(roundtrip, kind);
        }
    }
}

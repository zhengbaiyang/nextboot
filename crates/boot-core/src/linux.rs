//! Linux (Ubuntu) UEFI NVRAM backend via `/sys/firmware/efi/efivars` and systemd reboot.

use crate::classifier::classify_load_option;
use crate::parser::{
    encode_boot_order, encode_u16_le, format_boot_id, parse_boot_id, parse_boot_order,
    parse_load_option, parse_u16_le, EFI_GLOBAL_VARIABLE_GUID_LINUX,
};
use crate::{BootBackend, BootEntry, BootError, BootState, FirmwareType, RebootOptions};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

const EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";
const EFI_DIR: &str = "/sys/firmware/efi";

// Standard UEFI variable attributes: NON_VOLATILE | BOOTSERVICE_ACCESS | RUNTIME_ACCESS
const EFI_VAR_ATTR_NV_BS_RT: u32 = 0x0000_0007;

// Linux ext2/efivarfs ioctl constants for immutable flag (`chattr -i`)
const FS_IOC_GETFLAGS: libc::c_ulong = 0x8008_6601;
const FS_IOC_SETFLAGS: libc::c_ulong = 0x4008_6602;
const FS_IMMUTABLE_FL: libc::c_long = 0x0000_0010;

#[derive(Debug, Default, Clone)]
pub struct LinuxBootBackend;

impl LinuxBootBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn is_root() -> bool {
        unsafe { libc::geteuid() == 0 }
    }

    fn efivar_path(name: &str) -> PathBuf {
        PathBuf::from(EFIVARS_DIR).join(format!("{name}-{EFI_GLOBAL_VARIABLE_GUID_LINUX}"))
    }

    fn clear_immutable_flag(path: &Path) -> Result<(), BootError> {
        if !path.exists() {
            return Ok(());
        }
        let file = OpenOptions::new().read(true).open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                BootError::PermissionDenied(format!(
                    "需要 root 权限以修改 UEFI 变量: {}",
                    path.display()
                ))
            } else {
                BootError::Io(format!("打开 {} 失败: {e}", path.display()))
            }
        })?;

        let fd = file.as_raw_fd();
        let mut flags: libc::c_long = 0;

        let get_res = unsafe { libc::ioctl(fd, FS_IOC_GETFLAGS as _, &mut flags) };
        if get_res == 0 && (flags & FS_IMMUTABLE_FL) != 0 {
            let new_flags = flags & !FS_IMMUTABLE_FL;
            let set_res = unsafe { libc::ioctl(fd, FS_IOC_SETFLAGS as _, &new_flags) };
            if set_res != 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::PermissionDenied {
                    return Err(BootError::PermissionDenied(
                        "需要 root 权限以解除 UEFI 变量只读保护标志 (immutable)。".to_string(),
                    ));
                }
                return Err(BootError::SystemError(format!(
                    "解除 immutable 标志失败: {err}"
                )));
            }
        }

        Ok(())
    }

    fn read_efivar(name: &str) -> Result<Option<Vec<u8>>, BootError> {
        let path = Self::efivar_path(name);
        match fs::read(&path) {
            Ok(bytes) => {
                if bytes.len() < 4 {
                    return Err(BootError::ParseError(format!(
                        "UEFI 变量 {} 内容不足 4 字节属性头",
                        path.display()
                    )));
                }
                Ok(Some(bytes[4..].to_vec()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Err(BootError::PermissionDenied(format!(
                    "无权读取 {}，请检查权限或使用 sudo。",
                    path.display()
                )))
            }
            Err(e) => Err(BootError::Io(format!(
                "读取 UEFI 变量 {} 失败: {e}",
                path.display()
            ))),
        }
    }

    fn write_efivar(name: &str, payload: &[u8]) -> Result<(), BootError> {
        if !Self::is_root() {
            return Err(BootError::PermissionDenied(
                "修改 UEFI 引导项需要 root 权限（请通过 pkexec 或 sudo 运行）。".to_string(),
            ));
        }

        let path = Self::efivar_path(name);
        Self::clear_immutable_flag(&path)?;

        let mut full = Vec::with_capacity(4 + payload.len());
        full.extend_from_slice(&EFI_VAR_ATTR_NV_BS_RT.to_le_bytes());
        full.extend_from_slice(payload);

        // efivarfs requires attributes + payload in a single write syscall
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    BootError::PermissionDenied(format!(
                        "无权写入 UEFI 变量 {}，需要 root 权限。",
                        path.display()
                    ))
                } else {
                    BootError::Io(format!("打开 UEFI 变量 {} 写入失败: {e}", path.display()))
                }
            })?;

        file.write_all(&full)
            .map_err(|e| BootError::Io(format!("写入 UEFI 变量 {} 失败: {e}", path.display())))?;

        Ok(())
    }

    fn delete_efivar(name: &str) -> Result<(), BootError> {
        if !Self::is_root() {
            return Err(BootError::PermissionDenied(
                "清除 BootNext 需要 root 权限（请通过 pkexec 或 sudo 运行）。".to_string(),
            ));
        }

        let path = Self::efivar_path(name);
        if !path.exists() {
            return Ok(());
        }

        Self::clear_immutable_flag(&path)?;
        fs::remove_file(&path)
            .map_err(|e| BootError::Io(format!("删除 UEFI 变量 {} 失败: {e}", path.display())))
    }

    fn discover_all_boot_ids() -> Vec<u16> {
        let mut ids = Vec::new();
        let suffix = format!("-{EFI_GLOBAL_VARIABLE_GUID_LINUX}");

        if let Ok(entries) = fs::read_dir(EFIVARS_DIR) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let Some(name_str) = file_name.to_str() else {
                    continue;
                };
                if let Some(prefix) = name_str.strip_suffix(&suffix) {
                    if prefix.len() == 8 && prefix.starts_with("Boot") {
                        if let Ok(id) = parse_boot_id(&prefix[4..]) {
                            ids.push(id);
                        }
                    }
                }
            }
        }
        ids
    }
}

impl BootBackend for LinuxBootBackend {
    fn detect_firmware(&self) -> FirmwareType {
        if Path::new(EFIVARS_DIR).is_dir() || Path::new(EFI_DIR).is_dir() {
            FirmwareType::Uefi
        } else {
            FirmwareType::LegacyBios
        }
    }

    fn read_state(&self) -> Result<BootState, BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Ok(BootState {
                firmware,
                host_os: crate::current_host_os().to_string(),
                is_mock: false,
                is_elevated: Self::is_root(),
                boot_current: None,
                boot_next: None,
                default_boot: None,
                boot_order: vec![],
                entries: vec![],
            });
        }

        let boot_current = match Self::read_efivar("BootCurrent")? {
            Some(bytes) => Some(parse_u16_le(&bytes)?),
            None => None,
        };
        let boot_next = match Self::read_efivar("BootNext")? {
            Some(bytes) => Some(parse_u16_le(&bytes)?),
            None => None,
        };
        let boot_order = match Self::read_efivar("BootOrder")? {
            Some(bytes) => parse_boot_order(&bytes)?,
            None => Vec::new(),
        };

        let default_boot = boot_order.first().copied();

        let mut seen = HashSet::new();
        let mut all_ids = Vec::new();
        for &id in &boot_order {
            if seen.insert(id) {
                all_ids.push(id);
            }
        }
        for extra in Self::discover_all_boot_ids() {
            if seen.insert(extra) {
                all_ids.push(extra);
            }
        }

        let mut entries = Vec::new();
        for id in all_ids {
            let var_name = format!("Boot{}", format_boot_id(id));
            if let Ok(Some(raw)) = Self::read_efivar(&var_name) {
                if let Ok(parsed) = parse_load_option(&raw) {
                    let cls = classify_load_option(&parsed);
                    let order_index = boot_order.iter().position(|&x| x == id);
                    entries.push(BootEntry {
                        id,
                        id_hex: format_boot_id(id),
                        description: parsed.description,
                        display_name: cls.display_name,
                        efi_path: parsed.device_path.file_path,
                        os_kind: cls.os_kind,
                        active: parsed.active,
                        is_primary_os: cls.is_primary_os,
                        is_current: boot_current == Some(id),
                        is_default: default_boot == Some(id),
                        is_next: boot_next == Some(id),
                        order_index,
                        risk_warning: cls.risk_warning,
                    });
                }
            }
        }

        entries.sort_by_key(|e| (e.order_index.unwrap_or(usize::MAX), e.id));

        Ok(BootState {
            firmware,
            host_os: crate::current_host_os().to_string(),
            is_mock: false,
            is_elevated: Self::is_root(),
            boot_current: boot_current.map(format_boot_id),
            boot_next: boot_next.map(format_boot_id),
            default_boot: default_boot.map(format_boot_id),
            boot_order: boot_order.iter().copied().map(format_boot_id).collect(),
            entries,
        })
    }

    fn set_boot_next(&self, id: u16) -> Result<(), BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(firmware));
        }
        let var_name = format!("Boot{}", format_boot_id(id));
        if Self::read_efivar(&var_name)?.is_none() {
            return Err(BootError::EntryNotFound(format_boot_id(id)));
        }
        let encoded = encode_u16_le(id);
        Self::write_efivar("BootNext", &encoded)
    }

    fn clear_boot_next(&self) -> Result<(), BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(firmware));
        }
        Self::delete_efivar("BootNext")
    }

    fn set_default_boot(&self, id: u16) -> Result<(), BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(firmware));
        }
        let var_name = format!("Boot{}", format_boot_id(id));
        if Self::read_efivar(&var_name)?.is_none() {
            return Err(BootError::EntryNotFound(format_boot_id(id)));
        }

        let mut order = match Self::read_efivar("BootOrder")? {
            Some(bytes) => parse_boot_order(&bytes)?,
            None => Vec::new(),
        };
        order.retain(|&existing| existing != id);
        order.insert(0, id);

        let encoded = encode_boot_order(&order);
        Self::write_efivar("BootOrder", &encoded)
    }

    fn reboot(&self, options: RebootOptions) -> Result<(), BootError> {
        let mut args = vec!["reboot"];
        if options.force {
            // Ignore systemd inhibitor locks so remote reboot won't stall
            args.push("-i");
        }

        let status = Command::new("systemctl").args(&args).status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => {
                // Fallback to `/sbin/reboot` if systemctl fails in minimal environments
                let fallback = Command::new("/sbin/reboot").status();
                match fallback {
                    Ok(fs) if fs.success() => Ok(()),
                    _ => Err(BootError::SystemError(format!(
                        "执行 systemctl {} 失败，退出码: {:?}",
                        args.join(" "),
                        s.code()
                    ))),
                }
            }
            Err(e) => Err(BootError::SystemError(format!(
                "无法调用 systemctl 重启系统: {e}"
            ))),
        }
    }
}

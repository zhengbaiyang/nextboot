//! Windows UEFI NVRAM and system reboot backend using Win32 APIs.

use crate::classifier::classify_load_option;
use crate::parser::{
    encode_boot_order, encode_u16_le, format_boot_id, parse_boot_order, parse_load_option,
    parse_u16_le, EFI_GLOBAL_VARIABLE_GUID_WINDOWS,
};
use crate::{BootBackend, BootEntry, BootError, BootState, FirmwareType, RebootOptions};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ENVVAR_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER,
    ERROR_NOT_ALL_ASSIGNED, ERROR_PRIVILEGE_NOT_HELD, ERROR_SUCCESS, HANDLE, LUID,
};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, SE_SYSTEM_ENVIRONMENT_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    TOKEN_QUERY,
};
use windows_sys::Win32::System::Shutdown::{
    InitiateSystemShutdownExW, SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
    SHTDN_REASON_MINOR_OTHER,
};
use windows_sys::Win32::System::SystemInformation::{
    FirmwareTypeBios, FirmwareTypeUefi, GetFirmwareType, FIRMWARE_TYPE,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::System::WindowsProgramming::{
    GetFirmwareEnvironmentVariableW, SetFirmwareEnvironmentVariableW,
};

#[derive(Debug, Default, Clone)]
pub struct WindowsBootBackend;

impl WindowsBootBackend {
    pub fn new() -> Self {
        Self
    }

    fn to_wide_null(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn enable_privilege(privilege_name: *const u16) -> Result<(), BootError> {
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            let ok = OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            );
            if ok == 0 {
                return Err(BootError::PermissionDenied(
                    "无法打开进程权限令牌，请以管理员身份运行。".to_string(),
                ));
            }

            let mut luid = LUID {
                LowPart: 0,
                HighPart: 0,
            };
            let lookup_ok = LookupPrivilegeValueW(std::ptr::null(), privilege_name, &mut luid);
            if lookup_ok == 0 {
                CloseHandle(token);
                return Err(BootError::SystemError(format!(
                    "LookupPrivilegeValueW 失败，错误码: {}",
                    GetLastError()
                )));
            }

            let tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };

            let adj_ok = AdjustTokenPrivileges(
                token,
                0,
                &tp,
                std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            let last_err = GetLastError();
            CloseHandle(token);

            if adj_ok == 0 || last_err == ERROR_NOT_ALL_ASSIGNED || last_err == ERROR_PRIVILEGE_NOT_HELD {
                return Err(BootError::PermissionDenied(
                    "需要 Windows 管理员权限以访问 UEFI 固件变量或执行重启。".to_string(),
                ));
            }

            if last_err != ERROR_SUCCESS {
                return Err(BootError::SystemError(format!(
                    "AdjustTokenPrivileges 失败，错误码: {last_err}"
                )));
            }

            Ok(())
        }
    }

    fn read_uefi_var(name: &str) -> Result<Option<Vec<u8>>, BootError> {
        let wide_name = Self::to_wide_null(name);
        let wide_guid = Self::to_wide_null(EFI_GLOBAL_VARIABLE_GUID_WINDOWS);

        let mut buf_size = 4096usize;
        for _ in 0..4 {
            let mut buf = vec![0u8; buf_size];
            let bytes_read = unsafe {
                GetFirmwareEnvironmentVariableW(
                    wide_name.as_ptr(),
                    wide_guid.as_ptr(),
                    buf.as_mut_ptr() as *mut _,
                    buf.len() as u32,
                )
            };

            if bytes_read > 0 {
                buf.truncate(bytes_read as usize);
                return Ok(Some(buf));
            }

            let err = unsafe { GetLastError() };
            match err {
                ERROR_ENVVAR_NOT_FOUND => return Ok(None),
                ERROR_INSUFFICIENT_BUFFER => {
                    buf_size *= 2;
                    continue;
                }
                ERROR_PRIVILEGE_NOT_HELD => {
                    return Err(BootError::PermissionDenied(
                        "缺少 SeSystemEnvironmentPrivilege 权限，请以管理员身份运行。".to_string(),
                    ));
                }
                _ => {
                    return Err(BootError::SystemError(format!(
                        "读取 UEFI 变量 {name} 失败，Win32 错误码: {err}"
                    )));
                }
            }
        }

        Err(BootError::SystemError(format!(
            "读取 UEFI 变量 {name} 时缓冲区溢出"
        )))
    }

    fn write_uefi_var(name: &str, data: &[u8]) -> Result<(), BootError> {
        Self::enable_privilege(SE_SYSTEM_ENVIRONMENT_NAME)?;
        let wide_name = Self::to_wide_null(name);
        let wide_guid = Self::to_wide_null(EFI_GLOBAL_VARIABLE_GUID_WINDOWS);

        let ptr = if data.is_empty() {
            std::ptr::null()
        } else {
            data.as_ptr() as *const _
        };

        let ok = unsafe {
            SetFirmwareEnvironmentVariableW(
                wide_name.as_ptr(),
                wide_guid.as_ptr(),
                ptr,
                data.len() as u32,
            )
        };

        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_PRIVILEGE_NOT_HELD {
                return Err(BootError::PermissionDenied(
                    "缺少管理员权限，无法修改 UEFI 引导变量。".to_string(),
                ));
            }
            if data.is_empty() && err == ERROR_ENVVAR_NOT_FOUND {
                return Ok(());
            }
            return Err(BootError::SystemError(format!(
                "写入 UEFI 变量 {name} 失败，Win32 错误码: {err}"
            )));
        }

        Ok(())
    }

    fn read_boot_entry(
        id: u16,
        boot_current: Option<u16>,
        default_boot: Option<u16>,
        boot_next: Option<u16>,
        order_index: Option<usize>,
    ) -> Result<Option<BootEntry>, BootError> {
        let var_name = format!("Boot{}", format_boot_id(id));
        let Some(raw) = Self::read_uefi_var(&var_name)? else {
            return Ok(None);
        };

        let parsed = parse_load_option(&raw)?;
        let cls = classify_load_option(&parsed);

        Ok(Some(BootEntry {
            id,
            id_hex: format_boot_id(id),
            description: parsed.description,
            display_name: cls.display_name,
            efi_path: parsed.device_path.file_path,
            os_kind: cls.os_kind,
            os_label: cls.os_label,
            active: parsed.active,
            is_primary_os: cls.is_primary_os,
            is_current: boot_current == Some(id),
            is_default: default_boot == Some(id),
            is_next: boot_next == Some(id),
            order_index,
            risk_warning: cls.risk_warning,
        }))
    }
}

impl BootBackend for WindowsBootBackend {
    fn detect_firmware(&self) -> FirmwareType {
        let mut fw_type: FIRMWARE_TYPE = 0;
        let ok = unsafe { GetFirmwareType(&mut fw_type) };
        if ok == 0 {
            return FirmwareType::Unknown;
        }
        if fw_type == FirmwareTypeUefi {
            FirmwareType::Uefi
        } else if fw_type == FirmwareTypeBios {
            FirmwareType::LegacyBios
        } else {
            FirmwareType::Unknown
        }
    }

    fn read_state(&self) -> Result<BootState, BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Ok(BootState {
                firmware,
                host_os: crate::current_host_os().to_string(),
                is_mock: false,
                is_elevated: Self::enable_privilege(SE_SYSTEM_ENVIRONMENT_NAME).is_ok(),
                boot_current: None,
                boot_next: None,
                default_boot: None,
                boot_order: vec![],
                entries: vec![],
            });
        }

        Self::enable_privilege(SE_SYSTEM_ENVIRONMENT_NAME)?;

        let boot_current = match Self::read_uefi_var("BootCurrent")? {
            Some(bytes) => Some(parse_u16_le(&bytes)?),
            None => None,
        };

        let boot_next = match Self::read_uefi_var("BootNext")? {
            Some(bytes) => Some(parse_u16_le(&bytes)?),
            None => None,
        };

        let boot_order = match Self::read_uefi_var("BootOrder")? {
            Some(bytes) => parse_boot_order(&bytes)?,
            None => Vec::new(),
        };

        let default_boot = boot_order.first().copied();

        let mut seen = HashSet::new();
        let mut candidate_ids = Vec::new();
        for &id in &boot_order {
            if seen.insert(id) {
                candidate_ids.push(id);
            }
        }
        for extra in [boot_current, boot_next].into_iter().flatten() {
            if seen.insert(extra) {
                candidate_ids.push(extra);
            }
        }

        let mut entries = Vec::new();
        for id in candidate_ids {
            let order_index = boot_order.iter().position(|&x| x == id);
            match Self::read_boot_entry(id, boot_current, default_boot, boot_next, order_index) {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => {}
                Err(_) => {
                    // Skip malformed vendor-specific entries rather than breaking the whole list
                }
            }
        }

        entries.sort_by_key(|e| (e.order_index.unwrap_or(usize::MAX), e.id));

        Ok(BootState {
            firmware,
            host_os: crate::current_host_os().to_string(),
            is_mock: false,
            is_elevated: true,
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
        Self::enable_privilege(SE_SYSTEM_ENVIRONMENT_NAME)?;

        // Verify target entry actually exists before writing BootNext
        let var_name = format!("Boot{}", format_boot_id(id));
        if Self::read_uefi_var(&var_name)?.is_none() {
            return Err(BootError::EntryNotFound(format_boot_id(id)));
        }

        let encoded = encode_u16_le(id);
        Self::write_uefi_var("BootNext", &encoded)
    }

    fn clear_boot_next(&self) -> Result<(), BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(firmware));
        }
        Self::write_uefi_var("BootNext", &[])
    }

    fn set_default_boot(&self, id: u16) -> Result<(), BootError> {
        let firmware = self.detect_firmware();
        if firmware != FirmwareType::Uefi {
            return Err(BootError::UnsupportedFirmware(firmware));
        }
        Self::enable_privilege(SE_SYSTEM_ENVIRONMENT_NAME)?;

        let var_name = format!("Boot{}", format_boot_id(id));
        if Self::read_uefi_var(&var_name)?.is_none() {
            return Err(BootError::EntryNotFound(format_boot_id(id)));
        }

        let mut order = match Self::read_uefi_var("BootOrder")? {
            Some(bytes) => parse_boot_order(&bytes)?,
            None => Vec::new(),
        };

        order.retain(|&existing| existing != id);
        order.insert(0, id);

        let encoded = encode_boot_order(&order);
        Self::write_uefi_var("BootOrder", &encoded)
    }

    fn reboot(&self, options: RebootOptions) -> Result<(), BootError> {
        Self::enable_privilege(SE_SHUTDOWN_NAME)?;
        let message = Self::to_wide_null("NextBoot 正在重启以切换到目标操作系统");
        let reason = SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_MINOR_OTHER | SHTDN_REASON_FLAG_PLANNED;

        let ok = unsafe {
            InitiateSystemShutdownExW(
                std::ptr::null(),
                message.as_ptr() as *mut u16,
                0,
                if options.force { 1 } else { 0 },
                1, // bRebootAfterShutdown = TRUE
                reason,
            )
        };

        if ok == 0 {
            let err = unsafe { GetLastError() };
            return Err(BootError::SystemError(format!(
                "触发 Windows 重启失败，Win32 错误码: {err}"
            )));
        }

        Ok(())
    }
}

//! Standard UEFI `EFI_LOAD_OPTION` and NVRAM variable binary parsers.
//!
//! Reference: UEFI Specification 2.10, Section 3.1.3 (Load Options) & Section 10 (Device Path Protocol).

use crate::BootError;

pub const EFI_GLOBAL_VARIABLE_GUID_WINDOWS: &str = "{8BE4DF61-93CA-11D2-AA0D-00E098032B8C}";
pub const EFI_GLOBAL_VARIABLE_GUID_LINUX: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

pub const LOAD_OPTION_ACTIVE: u32 = 0x0000_0001;
pub const LOAD_OPTION_HIDDEN: u32 = 0x0000_0008;
pub const LOAD_OPTION_CATEGORY_MASK: u32 = 0x0000_1F00;
pub const LOAD_OPTION_CATEGORY_APP: u32 = 0x0000_0100;

/// Summary of EFI Device Path nodes attached to a boot entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevicePathSummary {
    pub file_path: Option<String>,
    pub has_hard_drive: bool,
    pub has_usb: bool,
    pub has_network: bool,
    pub has_cdrom: bool,
    pub has_firmware_volume: bool,
    pub has_bbs_legacy: bool,
}

/// Parsed fields of a UEFI `BootXXXX` (`EFI_LOAD_OPTION`) variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLoadOption {
    pub attributes: u32,
    pub active: bool,
    pub hidden: bool,
    pub is_category_app: bool,
    pub description: String,
    pub device_path: DevicePathSummary,
}

/// Format a 16-bit UEFI boot ID as uppercase 4-character hex (e.g. `0x000A` -> `"000A"`).
pub fn format_boot_id(id: u16) -> String {
    format!("{id:04X}")
}

/// Parse a boot ID from strings like `"0001"`, `"Boot0001"`, `"0x0001"`, `"1"`.
pub fn parse_boot_id(input: &str) -> Result<u16, BootError> {
    let trimmed = input.trim();
    let stripped = trimmed
        .strip_prefix("Boot")
        .or_else(|| trimmed.strip_prefix("boot"))
        .or_else(|| trimmed.strip_prefix("0x"))
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if stripped.is_empty() || stripped.len() > 4 {
        return Err(BootError::InvalidBootId(input.to_string()));
    }

    u16::from_str_radix(stripped, 16).map_err(|_| BootError::InvalidBootId(input.to_string()))
}

/// Parse a 2-byte little-endian `u16` UEFI variable (`BootCurrent`, `BootNext`).
pub fn parse_u16_le(bytes: &[u8]) -> Result<u16, BootError> {
    if bytes.len() < 2 {
        return Err(BootError::ParseError(format!(
            "expected at least 2 bytes for u16, got {}",
            bytes.len()
        )));
    }
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Serialize a 16-bit boot ID into little-endian bytes.
pub fn encode_u16_le(id: u16) -> [u8; 2] {
    id.to_le_bytes()
}

/// Parse `BootOrder` (packed array of little-endian `u16` boot IDs).
pub fn parse_boot_order(bytes: &[u8]) -> Result<Vec<u16>, BootError> {
    if bytes.len() % 2 != 0 {
        return Err(BootError::ParseError(format!(
            "BootOrder byte length must be even, got {}",
            bytes.len()
        )));
    }
    let mut order = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        order.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(order)
}

/// Encode `BootOrder` as packed little-endian `u16` bytes.
pub fn encode_boot_order(order: &[u16]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(order.len() * 2);
    for &id in order {
        buf.extend_from_slice(&id.to_le_bytes());
    }
    buf
}

/// Parse a raw `EFI_LOAD_OPTION` binary blob into structured metadata.
pub fn parse_load_option(bytes: &[u8]) -> Result<ParsedLoadOption, BootError> {
    // Minimum header: Attributes (4) + FilePathListLength (2) + null UTF-16 char (2) = 8 bytes
    if bytes.len() < 8 {
        return Err(BootError::ParseError(format!(
            "EFI_LOAD_OPTION too short: {} bytes",
            bytes.len()
        )));
    }

    let attributes = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let file_path_list_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;

    // Read null-terminated UTF-16LE Description starting at offset 6
    let mut offset = 6usize;
    let mut utf16_units = Vec::new();
    let mut found_null = false;

    while offset + 1 < bytes.len() {
        let unit = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        if unit == 0 {
            found_null = true;
            break;
        }
        utf16_units.push(unit);
    }

    if !found_null {
        return Err(BootError::ParseError(
            "EFI_LOAD_OPTION description missing null terminator".to_string(),
        ));
    }

    let description = String::from_utf16_lossy(&utf16_units).trim().to_string();

    let dp_end = offset
        .saturating_add(file_path_list_len)
        .min(bytes.len());
    let device_path = if offset < dp_end {
        parse_device_path_list(&bytes[offset..dp_end])
    } else {
        DevicePathSummary::default()
    };

    Ok(ParsedLoadOption {
        attributes,
        active: (attributes & LOAD_OPTION_ACTIVE) != 0,
        hidden: (attributes & LOAD_OPTION_HIDDEN) != 0,
        is_category_app: (attributes & LOAD_OPTION_CATEGORY_MASK) == LOAD_OPTION_CATEGORY_APP,
        description,
        device_path,
    })
}

fn parse_device_path_list(mut slice: &[u8]) -> DevicePathSummary {
    let mut summary = DevicePathSummary::default();

    while slice.len() >= 4 {
        let dp_type = slice[0];
        let dp_subtype = slice[1];
        let dp_len = u16::from_le_bytes([slice[2], slice[3]]) as usize;

        if dp_len < 4 || dp_len > slice.len() {
            break;
        }

        let payload = &slice[4..dp_len];

        match (dp_type, dp_subtype) {
            // End of Hardware Device Path (End Entire Device Path)
            (0x7F, 0xFF) => break,
            // Messaging Device Path (0x03)
            (0x03, 0x05) | (0x03, 0x0F) | (0x03, 0x10) => {
                // USB, USB Class, USB WWID
                summary.has_usb = true;
            }
            (0x03, 0x0B) | (0x03, 0x0C) | (0x03, 0x0D) | (0x03, 0x18) | (0x03, 0x1B) => {
                // MAC Address, IPv4, IPv6, URI/HTTP, Wi-Fi
                summary.has_network = true;
            }
            // Media Device Path (0x04)
            (0x04, 0x01) => {
                // Hard Drive (GPT/MBR partition)
                summary.has_hard_drive = true;
            }
            (0x04, 0x02) => {
                // CD-ROM
                summary.has_cdrom = true;
            }
            (0x04, 0x04) => {
                // File Path Media Device Path (UTF-16LE string)
                let path = decode_utf16_le_trimmed(payload);
                if !path.is_empty() {
                    match &mut summary.file_path {
                        Some(existing) => {
                            if !existing.ends_with('\\') && !path.starts_with('\\') {
                                existing.push('\\');
                            }
                            existing.push_str(&path);
                        }
                        None => summary.file_path = Some(path),
                    }
                }
            }
            (0x04, 0x06) | (0x04, 0x07) => {
                // PIWG Firmware File / PIWG Firmware Volume
                summary.has_firmware_volume = true;
            }
            // BIOS Boot Specification (0x05)
            (0x05, _) => {
                summary.has_bbs_legacy = true;
            }
            _ => {}
        }

        slice = &slice[dp_len..];
    }

    summary
}

fn decode_utf16_le_trimmed(bytes: &[u8]) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let u = u16::from_le_bytes([chunk[0], chunk[1]]);
        if u == 0 {
            break;
        }
        units.push(u);
    }
    String::from_utf16_lossy(&units).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_load_option(attrs: u32, desc: &str, dp_nodes: &[Vec<u8>]) -> Vec<u8> {
        let mut dp = Vec::new();
        for node in dp_nodes {
            dp.extend_from_slice(node);
        }
        // Append End Entire Device Path node
        dp.extend_from_slice(&[0x7F, 0xFF, 0x04, 0x00]);

        let mut out = Vec::new();
        out.extend_from_slice(&attrs.to_le_bytes());
        out.extend_from_slice(&(dp.len() as u16).to_le_bytes());
        for u in desc.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&dp);
        out
    }

    fn file_path_node(path: &str) -> Vec<u8> {
        let mut payload = Vec::new();
        for u in path.encode_utf16() {
            payload.extend_from_slice(&u.to_le_bytes());
        }
        payload.extend_from_slice(&0u16.to_le_bytes());
        let len = (4 + payload.len()) as u16;
        let mut node = vec![0x04, 0x04];
        node.extend_from_slice(&len.to_le_bytes());
        node.extend_from_slice(&payload);
        node
    }

    #[test]
    fn parses_ubuntu_load_option() {
        let hd_node = vec![0x04, 0x01, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00];
        let fp_node = file_path_node("\\EFI\\ubuntu\\shimx64.efi");
        let raw = build_load_option(LOAD_OPTION_ACTIVE, "ubuntu", &[hd_node, fp_node]);

        let parsed = parse_load_option(&raw).expect("valid load option");
        assert!(parsed.active);
        assert!(!parsed.hidden);
        assert!(!parsed.is_category_app);
        assert_eq!(parsed.description, "ubuntu");
        assert!(parsed.device_path.has_hard_drive);
        assert_eq!(
            parsed.device_path.file_path.as_deref(),
            Some("\\EFI\\ubuntu\\shimx64.efi")
        );
    }

    #[test]
    fn parses_pxe_network_load_option() {
        let net_node = vec![0x03, 0x0C, 0x06, 0x00, 0x00, 0x00];
        let raw = build_load_option(LOAD_OPTION_ACTIVE, "UEFI: PXE IPv4 Realtek", &[net_node]);

        let parsed = parse_load_option(&raw).expect("valid load option");
        assert!(parsed.device_path.has_network);
        assert!(!parsed.device_path.has_hard_drive);
    }

    #[test]
    fn boot_id_roundtrip() {
        assert_eq!(parse_boot_id("000A").unwrap(), 0x000A);
        assert_eq!(parse_boot_id("Boot0002").unwrap(), 0x0002);
        assert_eq!(parse_boot_id("0x1b").unwrap(), 0x001B);
        assert_eq!(format_boot_id(0x000A), "000A");
        assert!(parse_boot_id("ZZZZ").is_err());
    }
}

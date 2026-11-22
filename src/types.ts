export type FirmwareType = "uefi" | "legacy_bios" | "unknown";

export type OsKind =
  | "windows"
  | "ubuntu"
  | "debian"
  | "fedora"
  | "arch"
  | "linux"
  | "mac_os"
  | "network"
  | "usb"
  | "optical"
  | "firmware"
  | "unknown";

export type SwitchMode = "next" | "default";

export interface BootEntry {
  id: number;
  id_hex: string;
  description: string;
  display_name: string;
  efi_path: string | null;
  os_kind: OsKind;
  active: boolean;
  is_primary_os: boolean;
  is_current: boolean;
  is_default: boolean;
  is_next: boolean;
  order_index: number | null;
  risk_warning: string | null;
}

export interface BootState {
  firmware: FirmwareType;
  host_os: string;
  is_mock: boolean;
  is_elevated: boolean;
  boot_current: string | null;
  boot_next: string | null;
  default_boot: string | null;
  boot_order: string[];
  entries: BootEntry[];
}

export interface CommandResult {
  message: string;
  state: BootState | null;
}

export interface ApiError {
  code: string;
  message: string;
}

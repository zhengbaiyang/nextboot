export type FirmwareType = "uefi" | "legacy_bios" | "unknown";

export type OsKind =
  | "windows"
  | "ubuntu"
  | "debian"
  | "fedora"
  | "arch"
  | "manjaro"
  | "linux_mint"
  | "pop_os"
  | "opensuse"
  | "kali"
  | "redhat"
  | "nixos"
  | "gentoo"
  | "deepin"
  | "kylin"
  | "endeavour"
  | "zorin"
  | "elementary"
  | "steamos"
  | "proxmox"
  | "alpine"
  | "void_linux"
  | "linux"
  | "mac_os"
  | "network"
  | "usb"
  | "optical"
  | "firmware"
  | "unknown";

export type SwitchMode = "next" | "default";

export type ThemeMode = "light" | "dark";

export type ActiveTab = "boot" | "update";

export type EntryFilter = "all" | "os" | "advanced";

export interface BootEntry {
  id: number;
  id_hex: string;
  description: string;
  display_name: string;
  efi_path: string | null;
  os_kind: OsKind;
  os_label?: string;
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

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  release_name: string;
  release_notes: string;
  published_at: string | null;
  html_url: string;
  asset_name: string | null;
  asset_size: number | null;
  download_url: string | null;
  is_mock: boolean;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  stage: "downloading" | "installing" | "done" | string;
  message: string;
}

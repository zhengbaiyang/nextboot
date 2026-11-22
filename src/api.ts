import { invoke } from "@tauri-apps/api/core";
import type { ApiError, BootState, CommandResult, SwitchMode } from "./types";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Browser-only preview mock state (used only when running `npm run dev` in a regular browser)
let browserMockState: BootState = {
  firmware: "uefi",
  host_os: "windows",
  is_mock: true,
  is_elevated: true,
  boot_current: "0002",
  boot_next: null,
  default_boot: "0001",
  boot_order: ["0001", "0002", "0003", "0004"],
  entries: [
    {
      id: 1,
      id_hex: "0001",
      description: "ubuntu",
      display_name: "Ubuntu",
      efi_path: "\\EFI\\ubuntu\\shimx64.efi",
      os_kind: "ubuntu",
      active: true,
      is_primary_os: true,
      is_current: false,
      is_default: true,
      is_next: false,
      order_index: 0,
      risk_warning: null,
    },
    {
      id: 2,
      id_hex: "0002",
      description: "Windows Boot Manager",
      display_name: "Windows",
      efi_path: "\\EFI\\Microsoft\\Boot\\bootmgfw.efi",
      os_kind: "windows",
      active: true,
      is_primary_os: true,
      is_current: true,
      is_default: false,
      is_next: false,
      order_index: 1,
      risk_warning: null,
    },
    {
      id: 3,
      id_hex: "0003",
      description: "UEFI: PXE IPv4 Realtek PCIe GBE",
      display_name: "UEFI: PXE IPv4 Realtek PCIe GBE",
      efi_path: null,
      os_kind: "network",
      active: true,
      is_primary_os: false,
      is_current: false,
      is_default: false,
      is_next: false,
      order_index: 2,
      risk_warning: "这是网络启动项 (PXE/HTTP)，远控选择可能导致开机卡死失联。",
    },
    {
      id: 4,
      id_hex: "0004",
      description: "UEFI: Built-in EFI Shell",
      display_name: "UEFI: Built-in EFI Shell",
      efi_path: null,
      os_kind: "firmware",
      active: true,
      is_primary_os: false,
      is_current: false,
      is_default: false,
      is_next: false,
      order_index: 3,
      risk_warning: "这是主板固件或 UEFI Shell 工具，不会进入操作系统。",
    },
  ],
};

function syncBrowserMockFlags(): BootState {
  const defaultId = browserMockState.boot_order[0] ?? null;
  browserMockState = {
    ...browserMockState,
    default_boot: defaultId,
    entries: browserMockState.entries
      .map((e) => ({
        ...e,
        is_current: browserMockState.boot_current === e.id_hex,
        is_default: defaultId === e.id_hex,
        is_next: browserMockState.boot_next === e.id_hex,
        order_index: browserMockState.boot_order.indexOf(e.id_hex),
      }))
      .sort((a, b) => (a.order_index ?? 999) - (b.order_index ?? 999)),
  };
  return structuredClone(browserMockState);
}

export function normalizeError(err: unknown): ApiError {
  if (typeof err === "object" && err !== null && "message" in err) {
    const obj = err as { code?: unknown; message?: unknown };
    return {
      code: typeof obj.code === "string" ? obj.code : "unknown_error",
      message: typeof obj.message === "string" ? obj.message : String(err),
    };
  }
  return {
    code: "unknown_error",
    message: typeof err === "string" ? err : String(err),
  };
}

export async function getBootState(): Promise<BootState> {
  if (!isTauriRuntime()) {
    return syncBrowserMockFlags();
  }
  return invoke<BootState>("get_boot_state");
}

export async function applyBootSwitch(
  targetId: string,
  mode: SwitchMode,
  reboot: boolean,
  forceReboot: boolean
): Promise<CommandResult> {
  if (!isTauriRuntime()) {
    if (mode === "next") {
      browserMockState.boot_next = targetId;
    } else {
      browserMockState.boot_order = [
        targetId,
        ...browserMockState.boot_order.filter((id) => id !== targetId),
      ];
    }
    if (reboot) {
      const nextTarget = browserMockState.boot_next ?? browserMockState.boot_order[0] ?? null;
      browserMockState.boot_next = null;
      if (nextTarget) browserMockState.boot_current = nextTarget;
    }
    const state = syncBrowserMockFlags();
    return {
      message:
        mode === "next"
          ? `【浏览器模拟】已设置下次启动 Boot${targetId}${reboot ? " 并模拟重启" : ""}`
          : `【浏览器模拟】已将 Boot${targetId} 设为长期默认${reboot ? " 并模拟重启" : ""}`,
      state,
    };
  }

  return invoke<CommandResult>("apply_boot_switch", {
    targetId,
    mode,
    reboot,
    forceReboot,
  });
}

export async function clearBootNext(): Promise<CommandResult> {
  if (!isTauriRuntime()) {
    browserMockState.boot_next = null;
    const state = syncBrowserMockFlags();
    return { message: "【浏览器模拟】已清除单次启动指定 (BootNext)。", state };
  }
  return invoke<CommandResult>("clear_boot_next");
}

export async function rebootNow(force: boolean): Promise<CommandResult> {
  if (!isTauriRuntime()) {
    const nextTarget = browserMockState.boot_next ?? browserMockState.boot_order[0] ?? null;
    browserMockState.boot_next = null;
    if (nextTarget) browserMockState.boot_current = nextTarget;
    const state = syncBrowserMockFlags();
    return {
      message: force ? "【浏览器模拟】已执行强制重启。" : "【浏览器模拟】已执行重启。",
      state,
    };
  }
  return invoke<CommandResult>("reboot_now", { force });
}

export async function relaunchElevated(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>("relaunch_elevated");
}

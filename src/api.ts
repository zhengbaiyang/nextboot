import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ApiError,
  BootState,
  CommandResult,
  SwitchMode,
  UpdateInfo,
  UpdateProgress,
} from "./types";

export const APP_VERSION = "0.2.0";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Browser-only preview mock listeners for simulated update-progress events
const mockProgressListeners = new Set<(progress: UpdateProgress) => void>();

function emitMockProgress(progress: UpdateProgress) {
  for (const listener of mockProgressListeners) {
    listener(progress);
  }
}

// Browser-only preview mock state (used when running Vite preview/dev outside Tauri)
let browserMockState: BootState = {
  firmware: "uefi",
  host_os: "windows",
  is_mock: true,
  is_elevated: true,
  boot_current: "0002",
  boot_next: null,
  default_boot: "0001",
  boot_order: ["0001", "0002", "0003", "0004", "0005", "0006"],
  entries: [
    {
      id: 1,
      id_hex: "0001",
      description: "ubuntu",
      display_name: "Ubuntu",
      efi_path: "\\EFI\\ubuntu\\shimx64.efi",
      os_kind: "ubuntu",
      os_label: "Ubuntu",
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
      os_label: "Windows",
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
      description: "Arch Linux",
      display_name: "Arch Linux",
      efi_path: "\\EFI\\arch\\grubx64.efi",
      os_kind: "arch",
      os_label: "Arch Linux",
      active: true,
      is_primary_os: true,
      is_current: false,
      is_default: false,
      is_next: false,
      order_index: 2,
      risk_warning: null,
    },
    {
      id: 4,
      id_hex: "0004",
      description: "UnionTech OS Desktop",
      display_name: "Deepin / UOS",
      efi_path: "\\EFI\\deepin\\shimx64.efi",
      os_kind: "deepin",
      os_label: "Deepin / UOS",
      active: true,
      is_primary_os: true,
      is_current: false,
      is_default: false,
      is_next: false,
      order_index: 3,
      risk_warning: null,
    },
    {
      id: 5,
      id_hex: "0005",
      description: "UEFI: PXE IPv4 Realtek PCIe GBE",
      display_name: "UEFI: PXE IPv4 Realtek PCIe GBE",
      efi_path: null,
      os_kind: "network",
      os_label: "网络启动 (PXE)",
      active: true,
      is_primary_os: false,
      is_current: false,
      is_default: false,
      is_next: false,
      order_index: 4,
      risk_warning: "这是网络启动项 (PXE/HTTP)，远控选择可能导致开机卡死失联。",
    },
    {
      id: 6,
      id_hex: "0006",
      description: "UEFI: Built-in EFI Shell",
      display_name: "UEFI: Built-in EFI Shell",
      efi_path: null,
      os_kind: "firmware",
      os_label: "固件工具",
      active: true,
      is_primary_os: false,
      is_current: false,
      is_default: false,
      is_next: false,
      order_index: 5,
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
      const nextTarget =
        browserMockState.boot_next ?? browserMockState.boot_order[0] ?? null;
      browserMockState.boot_next = null;
      if (nextTarget) browserMockState.boot_current = nextTarget;
    }
    const state = syncBrowserMockFlags();
    return {
      message:
        mode === "next"
          ? `【模拟模式】已设置下次启动 Boot${targetId}${reboot ? " 并模拟重启" : ""}`
          : `【模拟模式】已将 Boot${targetId} 设为长期默认${reboot ? " 并模拟重启" : ""}`,
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
    return { message: "【模拟模式】已清除单次启动指定 (BootNext)。", state };
  }
  return invoke<CommandResult>("clear_boot_next");
}

export async function rebootNow(force: boolean): Promise<CommandResult> {
  if (!isTauriRuntime()) {
    const nextTarget =
      browserMockState.boot_next ?? browserMockState.boot_order[0] ?? null;
    browserMockState.boot_next = null;
    if (nextTarget) browserMockState.boot_current = nextTarget;
    const state = syncBrowserMockFlags();
    return {
      message: force ? "【模拟模式】已执行强制重启。" : "【模拟模式】已执行重启。",
      state,
    };
  }
  return invoke<CommandResult>("reboot_now", { force });
}

export async function relaunchElevated(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>("relaunch_elevated");
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  if (!isTauriRuntime()) {
    await new Promise((r) => window.setTimeout(r, 220));
    return {
      current_version: APP_VERSION,
      latest_version: "0.3.0",
      has_update: true,
      release_name: "NextBoot v0.3.0 — 远控引导与更新体验升级",
      release_notes: [
        "### 核心更新",
        "- 新增 20+ 主流 Linux 发行版 UEFI 识别与专属图标（Arch、Manjaro、Fedora、Deepin/UOS、银河麒麟、NixOS 等）",
        "- 全新 Bento 精密触感桌面界面，支持浅色 / 深色一键切换",
        "- 内置 GitHub Releases 应用内一键下载并安装更新，支持实时下载进度条",
        "- 强化远控防失联提示与非操作系统 UEFI 高级项筛选",
      ].join("\n"),
      published_at: "2026-11-20T10:30:00Z",
      html_url: "https://github.com/zhengbaiyang/nextboot/releases/latest",
      asset_name: "NextBoot_0.3.0_x64-setup.exe",
      asset_size: 8_942_150,
      download_url:
        "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot_0.3.0_x64-setup.exe",
      is_mock: true,
    };
  }

  return invoke<UpdateInfo>("check_for_updates");
}

export async function downloadAndInstallUpdate(
  downloadUrl: string,
  assetName: string
): Promise<string> {
  if (!isTauriRuntime()) {
    const total = 8_942_150;
    const steps: Array<{
      percent: number;
      stage: UpdateProgress["stage"];
      message: string;
      delay: number;
    }> = [
      { percent: 8, stage: "downloading", message: `正在连接 GitHub Releases 下载 ${assetName}...`, delay: 160 },
      { percent: 34, stage: "downloading", message: "正在下载更新包...", delay: 240 },
      { percent: 67, stage: "downloading", message: "正在下载更新包...", delay: 240 },
      { percent: 94, stage: "downloading", message: "正在校验更新包完整性...", delay: 220 },
      { percent: 100, stage: "installing", message: "下载完成，正在拉起安装程序...", delay: 320 },
      { percent: 100, stage: "done", message: "【模拟模式】安装程序已准备就绪（预览模式不退出应用）。", delay: 220 },
    ];

    for (const step of steps) {
      await new Promise((r) => window.setTimeout(r, step.delay));
      emitMockProgress({
        downloaded: Math.round((total * step.percent) / 100),
        total,
        percent: step.percent,
        stage: step.stage,
        message: step.message,
      });
    }
    return `【模拟模式】已完成 ${assetName} 下载并模拟启动安装。`;
  }

  return invoke<string>("download_and_install_update", {
    downloadUrl,
    assetName,
  });
}

export async function openReleasePage(url: string): Promise<void> {
  if (!isTauriRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  await invoke<void>("open_release_page", { url });
}

export async function subscribeUpdateProgress(
  handler: (progress: UpdateProgress) => void
): Promise<() => void> {
  if (!isTauriRuntime()) {
    mockProgressListeners.add(handler);
    return () => {
      mockProgressListeners.delete(handler);
    };
  }

  const unlisten = await listen<UpdateProgress>("update-progress", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

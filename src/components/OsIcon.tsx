import type { CSSProperties } from "react";
import type { OsKind } from "../types";

interface OsIconProps {
  kind: OsKind;
  size?: "sm" | "md";
}

interface OsVisualMeta {
  color: string;
  tint: string;
  border: string;
  fallbackLabel: string;
}

const OS_META: Record<OsKind, OsVisualMeta> = {
  windows: {
    color: "#0284c7",
    tint: "rgba(2, 132, 199, 0.12)",
    border: "rgba(2, 132, 199, 0.22)",
    fallbackLabel: "Windows",
  },
  ubuntu: {
    color: "#ea580c",
    tint: "rgba(234, 88, 12, 0.12)",
    border: "rgba(234, 88, 12, 0.24)",
    fallbackLabel: "Ubuntu",
  },
  debian: {
    color: "#d70a53",
    tint: "rgba(215, 10, 83, 0.12)",
    border: "rgba(215, 10, 83, 0.22)",
    fallbackLabel: "Debian",
  },
  fedora: {
    color: "#2952ff",
    tint: "rgba(41, 82, 255, 0.12)",
    border: "rgba(41, 82, 255, 0.24)",
    fallbackLabel: "Fedora",
  },
  arch: {
    color: "#0ea5e9",
    tint: "rgba(14, 165, 233, 0.13)",
    border: "rgba(14, 165, 233, 0.25)",
    fallbackLabel: "Arch Linux",
  },
  manjaro: {
    color: "#16a34a",
    tint: "rgba(22, 163, 74, 0.12)",
    border: "rgba(22, 163, 74, 0.24)",
    fallbackLabel: "Manjaro",
  },
  linux_mint: {
    color: "#4d9e2a",
    tint: "rgba(77, 158, 42, 0.13)",
    border: "rgba(77, 158, 42, 0.24)",
    fallbackLabel: "Linux Mint",
  },
  pop_os: {
    color: "#0d9488",
    tint: "rgba(13, 148, 136, 0.13)",
    border: "rgba(13, 148, 136, 0.24)",
    fallbackLabel: "Pop!_OS",
  },
  opensuse: {
    color: "#65a30d",
    tint: "rgba(101, 163, 13, 0.13)",
    border: "rgba(101, 163, 13, 0.24)",
    fallbackLabel: "openSUSE",
  },
  kali: {
    color: "#2563eb",
    tint: "rgba(37, 99, 235, 0.13)",
    border: "rgba(37, 99, 235, 0.24)",
    fallbackLabel: "Kali Linux",
  },
  redhat: {
    color: "#dc2626",
    tint: "rgba(220, 38, 38, 0.12)",
    border: "rgba(220, 38, 38, 0.24)",
    fallbackLabel: "Red Hat",
  },
  nixos: {
    color: "#4f46e5",
    tint: "rgba(79, 70, 229, 0.12)",
    border: "rgba(79, 70, 229, 0.24)",
    fallbackLabel: "NixOS",
  },
  gentoo: {
    color: "#7c3aed",
    tint: "rgba(124, 58, 237, 0.12)",
    border: "rgba(124, 58, 237, 0.24)",
    fallbackLabel: "Gentoo",
  },
  deepin: {
    color: "#0070f3",
    tint: "rgba(0, 112, 243, 0.12)",
    border: "rgba(0, 112, 243, 0.24)",
    fallbackLabel: "Deepin / UOS",
  },
  kylin: {
    color: "#e11d48",
    tint: "rgba(225, 29, 72, 0.12)",
    border: "rgba(225, 29, 72, 0.24)",
    fallbackLabel: "银河麒麟",
  },
  endeavour: {
    color: "#8b5cf6",
    tint: "rgba(139, 92, 246, 0.13)",
    border: "rgba(139, 92, 246, 0.25)",
    fallbackLabel: "EndeavourOS",
  },
  zorin: {
    color: "#0284c7",
    tint: "rgba(2, 132, 199, 0.12)",
    border: "rgba(2, 132, 199, 0.24)",
    fallbackLabel: "Zorin OS",
  },
  elementary: {
    color: "#2563eb",
    tint: "rgba(37, 99, 235, 0.12)",
    border: "rgba(37, 99, 235, 0.22)",
    fallbackLabel: "elementary OS",
  },
  steamos: {
    color: "#1d4ed8",
    tint: "rgba(29, 78, 216, 0.13)",
    border: "rgba(29, 78, 216, 0.25)",
    fallbackLabel: "SteamOS",
  },
  proxmox: {
    color: "#e57000",
    tint: "rgba(229, 112, 0, 0.13)",
    border: "rgba(229, 112, 0, 0.25)",
    fallbackLabel: "Proxmox VE",
  },
  alpine: {
    color: "#0369a1",
    tint: "rgba(3, 105, 161, 0.13)",
    border: "rgba(3, 105, 161, 0.25)",
    fallbackLabel: "Alpine Linux",
  },
  void_linux: {
    color: "#15803d",
    tint: "rgba(21, 128, 61, 0.13)",
    border: "rgba(21, 128, 61, 0.25)",
    fallbackLabel: "Void Linux",
  },
  linux: {
    color: "#059669",
    tint: "rgba(5, 150, 105, 0.12)",
    border: "rgba(5, 150, 105, 0.24)",
    fallbackLabel: "Linux",
  },
  mac_os: {
    color: "#475569",
    tint: "rgba(100, 116, 139, 0.14)",
    border: "rgba(100, 116, 139, 0.26)",
    fallbackLabel: "macOS",
  },
  network: {
    color: "#d97706",
    tint: "rgba(217, 119, 6, 0.13)",
    border: "rgba(217, 119, 6, 0.26)",
    fallbackLabel: "网络启动 (PXE)",
  },
  usb: {
    color: "#d97706",
    tint: "rgba(217, 119, 6, 0.13)",
    border: "rgba(217, 119, 6, 0.26)",
    fallbackLabel: "USB 设备",
  },
  optical: {
    color: "#d97706",
    tint: "rgba(217, 119, 6, 0.13)",
    border: "rgba(217, 119, 6, 0.26)",
    fallbackLabel: "光驱设备",
  },
  firmware: {
    color: "#b45309",
    tint: "rgba(217, 119, 6, 0.13)",
    border: "rgba(217, 119, 6, 0.26)",
    fallbackLabel: "固件工具",
  },
  unknown: {
    color: "#64748b",
    tint: "rgba(100, 116, 139, 0.12)",
    border: "rgba(100, 116, 139, 0.22)",
    fallbackLabel: "未知项",
  },
};

export function getFallbackOsLabel(kind: OsKind): string {
  return OS_META[kind]?.fallbackLabel ?? "未知项";
}

function renderSvg(kind: OsKind, px: number) {
  switch (kind) {
    case "windows":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="currentColor">
          <path d="M3 4.6 11.2 3.4V11.3H3V4.6Zm9.4-1.3L21 2V11.3H12.4V3.3ZM3 12.7H11.2V20.6L3 19.4V12.7Zm9.4 0H21V22L12.4 20.7V12.7Z" />
        </svg>
      );

    case "ubuntu":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="6.2" />
          <circle cx="18.3" cy="12" r="2.1" fill="currentColor" stroke="none" />
          <circle cx="8.8" cy="6.6" r="2.1" fill="currentColor" stroke="none" />
          <circle cx="8.8" cy="17.4" r="2.1" fill="currentColor" stroke="none" />
        </svg>
      );

    case "debian":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round">
          <path d="M13.3 13.8c-1.8.6-3.4-.5-3.4-2.3 0-2 1.6-3.4 3.6-3.4 2.7 0 4.6 2.1 4.6 4.8 0 3.7-3 6.6-6.8 6.6-4.2 0-7.3-3.2-7.3-7.4 0-4.8 3.8-8.3 8.6-8.3 3.1 0 5.9 1.6 7.4 4.1" />
        </svg>
      );

    case "fedora":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 3.5a8.5 8.5 0 1 1-8.5 8.5v5.5a3 3 0 0 0 3 3H12" />
          <path d="M12 16.5V9.8a2.6 2.6 0 0 1 5.2 0" />
          <path d="M9.5 12.2h4.7" />
        </svg>
      );

    case "arch":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="currentColor">
          <path d="M12 2.4c-.9 2.3-1.5 3.8-2.6 6 .7.7 1.5 1.5 2.8 2.4-1.4-.6-2.4-1.2-3.1-1.8-1.4 2.9-3.5 7-6.3 12.2 2.9-1.7 5.2-2.7 7.3-3.1-.1-.5-.2-1-.2-1.6 0-2 1-3.5 2.1-3.5s2.1 1.5 2.1 3.5c0 .6-.1 1.1-.2 1.6 2.1.4 4.4 1.4 7.3 3.1-1.3-2.5-2.5-4.8-3.6-7-1.1-.8-2.2-1.6-3.5-2.2 1.2.3 2.1.7 2.8 1.2C14.7 8.4 13.8 6.4 12 2.4Z" />
        </svg>
      );

    case "manjaro":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="currentColor">
          <path d="M3.5 3.5h10.2v4.3H8v12.7H3.5V3.5Zm6.2 6h4v11h-4v-11Zm6.2-6h4.6v17h-4.6v-17Z" />
        </svg>
      );

    case "linux_mint":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M4 5.5h10.5a5.5 5.5 0 0 1 5.5 5.5v3a5.5 5.5 0 0 1-5.5 5.5H9.5A5.5 5.5 0 0 1 4 14V5.5Z" />
          <path d="M8 9.2v6.1m3.2-2.5v2.5m0-2.5a1.9 1.9 0 0 1 3.8 0v2.5m-3.8-2.5a1.9 1.9 0 0 0-3.2 0" />
        </svg>
      );

    case "pop_os":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
          <path d="M6.5 16.5V6.5h4.2a3 3 0 0 1 0 6H6.5" />
          <path d="M16.8 6.5v6.2" />
          <circle cx="16.8" cy="16.2" r="1.2" fill="currentColor" stroke="none" />
          <path d="M5.5 19.5h13" />
        </svg>
      );

    case "opensuse":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M4.5 14.5c0-4.7 3.8-8 8.7-8 3.7 0 6.3 2.1 6.3 4.8 0 2.1-1.7 3.5-4.3 3.5H10" />
          <circle cx="14.6" cy="10.4" r="1.4" fill="currentColor" stroke="none" />
          <path d="M4.5 14.5c0 2.5 2 4.2 4.6 4.2h4.2" />
        </svg>
      );

    case "kali":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M4 16.5c2.2-1.5 5.4-2.2 8.8-1.4 2.7.6 5.2-.7 7.2-3.4-2.7.5-5 .1-6.8-1.2 1.8-1.1 3.7-2.7 5-5.3-3.2 1.7-6.4 2.2-9.5 1.8 1.5 1.3 2.3 2.8 2.1 4.4-2.4-.3-4.6.6-6.8 5.1Z" />
        </svg>
      );

    case "redhat":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M6.8 13.3 8.6 7.5c.3-.9 1.1-1.4 2-1.2l1.4.4 1.4-.4c.9-.2 1.7.3 2 1.2l1.8 5.8" />
          <path d="M3.2 14.8c2.3 2.2 5.4 3.4 8.8 3.4s6.5-1.2 8.8-3.4c-1.5-.8-3.5-1.3-5.8-1.4H9c-2.3.1-4.3.6-5.8 1.4Z" />
        </svg>
      );

    case "nixos":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 3.2v17.6M4.4 7.6l15.2 8.8M19.6 7.6 4.4 16.4" />
          <circle cx="12" cy="12" r="2.7" fill="var(--surface, #fff)" />
        </svg>
      );

    case "gentoo":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
          <path d="M4.8 16.2c0-4.9 4.5-10.7 9.4-10.7 2.8 0 5 1.9 5 4.4 0 3.7-4.4 8.6-9.8 8.6-2.7 0-4.6-1-4.6-2.3Z" />
          <circle cx="13.5" cy="10.4" r="1.7" fill="currentColor" stroke="none" />
        </svg>
      );

    case "deepin":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="8.2" />
          <path d="M14.8 6.8v6.6a3.3 3.3 0 1 1-3.3-3.3c1.2 0 2.3.6 3.3 1.7" />
        </svg>
      );

    case "kylin":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 3.2 19 6.6v5.2c0 4.5-3 7.6-7 9-4-1.4-7-4.5-7-9V6.6L12 3.2Z" />
          <path d="M9.5 8.5v7m0-3.5 5-3.5m-3.2 2.3 3.4 4.7" />
        </svg>
      );

    case "endeavour":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="currentColor">
          <path d="M12.5 3.5 5 17.8h12.2c2.1 0 3.3-1.4 2.4-3.3L12.5 3.5Zm-1.2 4.8 4.4 7.2H8.1l3.2-7.2Z" />
        </svg>
      );

    case "zorin":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M6.5 6.5h11l-11 11h11" />
          <path d="M4.5 12h2.2m10.6 0h2.2" />
        </svg>
      );

    case "elementary":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="8.2" />
          <path d="M8.5 13.8c3.6-.3 7.1-2.5 7.1-4.8 0-1.3-1.1-2.1-2.4-2.1-2.5 0-4.8 2.8-4.8 5.6 0 2.2 1.6 3.5 3.9 3.5 1.5 0 2.9-.6 4-1.6" />
        </svg>
      );

    case "steamos":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="8.5" />
          <circle cx="15" cy="9.5" r="2.2" />
          <circle cx="9.6" cy="15.2" r="1.7" />
          <path d="m13.3 11.2-2.4 2.5M4 13.2l4.1 1.6" />
        </svg>
      );

    case "proxmox":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
          <path d="m5.5 6.5 5 5.5-5 5.5" />
          <path d="m18.5 6.5-5 5.5 5 5.5" />
          <path d="m8.8 5.5 3.2 3.3 3.2-3.3M8.8 18.5l3.2-3.3 3.2 3.3" />
        </svg>
      );

    case "alpine":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
          <path d="m3.5 17.5 6.2-9.5 4.2 6.4 2.6-3.8 4 6.9H3.5Z" />
        </svg>
      );

    case "void_linux":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round">
          <path d="M6.4 7.6a7.5 7.5 0 0 1 11.2 1.2" />
          <path d="M17.6 16.4a7.5 7.5 0 0 1-11.2-1.2" />
          <circle cx="12" cy="12" r="2.6" fill="currentColor" stroke="none" />
        </svg>
      );

    case "linux":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3.5" y="4.5" width="17" height="15" rx="3" />
          <path d="m7.8 9.8 3 2.3-3 2.3M12.8 14.5h3.8" />
        </svg>
      );

    case "mac_os":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="currentColor">
          <path d="M16.4 12.7c0-2.1 1.7-3.1 1.8-3.2-1-1.5-2.5-1.7-3.1-1.7-1.3-.1-2.5.8-3.2.8-.7 0-1.7-.8-2.8-.7-1.4 0-2.7.8-3.5 2.1-1.5 2.6-.4 6.4 1.1 8.5.7 1 1.5 2.2 2.6 2.1 1.1 0 1.5-.7 2.8-.7 1.3 0 1.6.7 2.8.7 1.2 0 1.9-1 2.6-2 .8-1.2 1.2-2.4 1.2-2.5-.1 0-2.3-.9-2.3-3.4ZM14.3 6.3c.6-.7 1-1.7.9-2.7-.9 0-1.9.6-2.5 1.3-.5.6-1 1.6-.9 2.6 1 .1 2-.5 2.5-1.2Z" />
        </svg>
      );

    case "network":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round">
          <circle cx="12" cy="12" r="8.2" />
          <path d="M4.2 12h15.6M12 3.8c2.4 2.3 3.7 5.1 3.7 8.2s-1.3 5.9-3.7 8.2M12 3.8c-2.4 2.3-3.7 5.1-3.7 8.2s1.3 5.9 3.7 8.2" />
        </svg>
      );

    case "usb":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 3.2v15m0-15-2.5 3h5L12 3.2Zm0 7.2 4.2-2.2v3m-4.2 2-4.2-2.2v2.7" />
          <circle cx="12" cy="19.4" r="1.8" fill="currentColor" stroke="none" />
        </svg>
      );

    case "optical":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="1.9">
          <circle cx="12" cy="12" r="8.2" />
          <circle cx="12" cy="12" r="2.4" />
          <path d="M15.8 8.2 13.7 10.3" strokeLinecap="round" />
        </svg>
      );

    case "firmware":
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
          <rect x="7" y="7" width="10" height="10" rx="2" />
          <rect x="10" y="10" width="4" height="4" rx="0.6" />
          <path d="M9.5 3.8v3.2M14.5 3.8v3.2M9.5 17v3.2M14.5 17v3.2M3.8 9.5H7M3.8 14.5H7M17 9.5h3.2M17 14.5h3.2" />
        </svg>
      );

    default:
      return (
        <svg viewBox="0 0 24 24" width={px} height={px} fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round">
          <rect x="4" y="5" width="16" height="14" rx="3" />
          <path d="M8.5 9.8h7M8.5 14h4.5" />
        </svg>
      );
  }
}

export function OsIcon({ kind, size = "md" }: OsIconProps) {
  const meta = OS_META[kind] ?? OS_META.unknown;
  const px = size === "sm" ? 17 : 22;

  const style = {
    "--os-color": meta.color,
    "--os-tint": meta.tint,
    "--os-border": meta.border,
  } as CSSProperties;

  return (
    <div
      className={`os-icon ${size === "sm" ? "os-icon-sm" : "os-icon-md"}`}
      style={style}
      aria-hidden="true"
    >
      {renderSvg(kind, px)}
    </div>
  );
}

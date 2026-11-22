import type { OsKind } from "../types";

interface OsIconProps {
  kind: OsKind;
}

export function OsIcon({ kind }: OsIconProps) {
  switch (kind) {
    case "windows":
      return (
        <div className="os-icon os-icon-windows" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
            <path d="M3 4.5 11.2 3.3V11.3H3V4.5Zm9.3-1.4L21 1.8V11.3H12.3V3.1ZM3 12.6H11.2V20.6L3 19.4V12.6Zm9.3 0H21V22.1L12.3 20.8V12.6Z" />
          </svg>
        </div>
      );
    case "ubuntu":
      return (
        <div className="os-icon os-icon-ubuntu" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="6.2" />
            <circle cx="18.2" cy="12" r="1.9" fill="currentColor" stroke="none" />
            <circle cx="8.9" cy="6.6" r="1.9" fill="currentColor" stroke="none" />
            <circle cx="8.9" cy="17.4" r="1.9" fill="currentColor" stroke="none" />
          </svg>
        </div>
      );
    case "debian":
    case "fedora":
    case "arch":
    case "linux":
      return (
        <div className="os-icon os-icon-linux" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.9">
            <rect x="3" y="4" width="18" height="15" rx="2.5" />
            <path d="m7.5 10 3 2.5-3 2.5M12.5 15h4" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </div>
      );
    case "mac_os":
      return (
        <div className="os-icon os-icon-macos" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.9">
            <rect x="4" y="4" width="16" height="12" rx="2" />
            <path d="M9 20h6M12 16v4" strokeLinecap="round" />
          </svg>
        </div>
      );
    case "network":
      return (
        <div className="os-icon os-icon-warn" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8">
            <circle cx="12" cy="12" r="8.5" />
            <path d="M3.8 12h16.4M12 3.5c2.5 2.4 3.8 5.3 3.8 8.5s-1.3 6.1-3.8 8.5M12 3.5C9.5 5.9 8.2 8.8 8.2 12s1.3 6.1 3.8 8.5" />
          </svg>
        </div>
      );
    case "usb":
      return (
        <div className="os-icon os-icon-warn" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8">
            <path d="M12 3v15m0-15-2.5 3h5L12 3Zm0 7 4-2v3m-4 2-4-2v2.5" strokeLinecap="round" strokeLinejoin="round" />
            <circle cx="12" cy="19.5" r="1.8" fill="currentColor" stroke="none" />
          </svg>
        </div>
      );
    default:
      return (
        <div className="os-icon os-icon-neutral" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8">
            <rect x="4" y="5" width="16" height="14" rx="2.5" />
            <path d="M8 9h8M8 13h5" strokeLinecap="round" />
          </svg>
        </div>
      );
  }
}

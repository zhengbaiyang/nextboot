import type { FirmwareType } from "../types";

interface LegacyBiosNoticeProps {
  firmware: FirmwareType;
  hostOs: string;
}

export function LegacyBiosNotice({ firmware, hostOs }: LegacyBiosNoticeProps) {
  return (
    <section className="notice-panel" role="alert">
      <div className="notice-icon" aria-hidden="true">
        !
      </div>
      <h2>当前电脑未运行在 UEFI 引导模式</h2>
      <p>
        检测到主板固件模式为{" "}
        <code>{firmware === "legacy_bios" ? "Legacy BIOS (MBR)" : "未知固件"}</code>
        （当前系统：{hostOs === "windows" ? "Windows" : "Ubuntu / Linux"}）。
      </p>
      <p className="notice-sub">
        NextBoot v1 通过直接读写主板 UEFI NVRAM 来实现 Windows 与 Ubuntu 零侵入安全切换，因此当前暂不支持传统 Legacy BIOS 机器。
      </p>
      <div className="notice-tips">
        <div className="notice-tips-title">排查建议：</div>
        <ul>
          <li>Windows 下可按 Win+R 输入 <code>msinfo32</code>，查看「BIOS 模式」是否为「传统」。</li>
          <li>Ubuntu 下可检查 <code>/sys/firmware/efi</code> 目录是否存在。</li>
          <li>如主板支持 UEFI，建议使用 GPT 分区与 UEFI 引导安装双系统后再启用 NextBoot。</li>
        </ul>
      </div>
    </section>
  );
}

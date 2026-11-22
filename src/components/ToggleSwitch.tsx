interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  tone?: "cobalt" | "danger";
  id?: string;
  ariaLabel?: string;
}

export function ToggleSwitch({
  checked,
  onChange,
  disabled = false,
  tone = "cobalt",
  id,
  ariaLabel,
}: ToggleSwitchProps) {
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`tactile-switch ${checked ? "is-checked" : ""} ${
        tone === "danger" ? "tactile-switch-danger" : ""
      }`}
    >
      <span className="tactile-switch-thumb" aria-hidden="true" />
    </button>
  );
}

interface PathFieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onBrowse: () => void;
  disabled?: boolean;
}

export function PathField({
  label,
  value,
  onChange,
  onBrowse,
  disabled = false,
}: PathFieldProps) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <div className="field-row">
        <input
          type="text"
          value={value}
          onChange={(event) => onChange(event.currentTarget.value)}
          disabled={disabled}
          spellCheck={false}
        />
        <button type="button" onClick={onBrowse} disabled={disabled}>
          Browse
        </button>
      </div>
    </label>
  );
}

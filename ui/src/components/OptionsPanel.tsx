interface OptionsPanelProps {
  dialect: string;
  allDialects: boolean;
  runtime: boolean;
  examples: boolean;
  onDialectChange: (value: string) => void;
  onAllDialectsChange: (value: boolean) => void;
  onRuntimeChange: (value: boolean) => void;
  onExamplesChange: (value: boolean) => void;
  disabled?: boolean;
}

export function OptionsPanel({
  dialect,
  allDialects,
  runtime,
  examples,
  onDialectChange,
  onAllDialectsChange,
  onRuntimeChange,
  onExamplesChange,
  disabled = false,
}: OptionsPanelProps) {
  return (
    <fieldset className="options-panel" disabled={disabled}>
      <legend>Options</legend>
      <label className="field inline">
        <span className="field-label">Dialect filter</span>
        <input
          type="text"
          value={dialect}
          onChange={(event) => onDialectChange(event.currentTarget.value)}
          disabled={disabled || allDialects}
        />
      </label>
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={allDialects}
          onChange={(event) => onAllDialectsChange(event.currentTarget.checked)}
          disabled={disabled}
        />
        <span>All dialects</span>
      </label>
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={runtime}
          onChange={(event) => onRuntimeChange(event.currentTarget.checked)}
          disabled={disabled}
        />
        <span>Runtime</span>
      </label>
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={examples}
          onChange={(event) => onExamplesChange(event.currentTarget.checked)}
          disabled={disabled}
        />
        <span>Examples</span>
      </label>
    </fieldset>
  );
}

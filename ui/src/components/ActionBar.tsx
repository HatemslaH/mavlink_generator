interface ActionBarProps {
  busy: boolean;
  onValidate: () => void;
  onGenerate: () => void;
}

export function ActionBar({ busy, onValidate, onGenerate }: ActionBarProps) {
  return (
    <div className="action-bar">
      <button type="button" onClick={onValidate} disabled={busy}>
        Validate
      </button>
      <button
        type="button"
        className="primary"
        onClick={onGenerate}
        disabled={busy}
      >
        Generate
      </button>
    </div>
  );
}

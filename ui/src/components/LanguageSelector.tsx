import type { LanguageInfo } from "../api/commands";

interface LanguageSelectorProps {
  languages: LanguageInfo[];
  selected: Set<string>;
  onChange: (selected: Set<string>) => void;
  disabled?: boolean;
}

export function LanguageSelector({
  languages,
  selected,
  onChange,
  disabled = false,
}: LanguageSelectorProps) {
  const allSelected =
    languages.length > 0 && selected.size === languages.length;

  function toggleLanguage(id: string) {
    const next = new Set(selected);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    onChange(next);
  }

  function toggleAll() {
    if (allSelected) {
      onChange(new Set());
    } else {
      onChange(new Set(languages.map((language) => language.id)));
    }
  }

  return (
    <fieldset className="language-selector" disabled={disabled}>
      <legend>Languages</legend>
      <div className="language-toolbar">
        <button type="button" onClick={toggleAll} disabled={disabled}>
          {allSelected ? "Deselect all" : "Select all"}
        </button>
      </div>
      <div className="language-grid">
        {languages.map((language) => (
          <label key={language.id} className="language-option">
            <input
              type="checkbox"
              checked={selected.has(language.id)}
              onChange={() => toggleLanguage(language.id)}
              disabled={disabled}
            />
            <span>{language.display_name}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

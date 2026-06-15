import { useEffect, useState } from "react";
import {
  defaultPaths,
  formatValidateResult,
  generate,
  listenGenerateProgress,
  listLanguages,
  pickOutputDir,
  pickXmlFile,
  validateDialects,
  type GenerateOptions,
  type LanguageInfo,
  type LogLine,
} from "./api/commands";
import { ActionBar } from "./components/ActionBar";
import { LanguageSelector } from "./components/LanguageSelector";
import { LogPanel } from "./components/LogPanel";
import { OptionsPanel } from "./components/OptionsPanel";
import { PathField } from "./components/PathField";
import "./App.css";

const DEFAULT_DIALECT = "rt_rc";

function appendLog(
  lines: LogLine[],
  text: string,
  kind: LogLine["kind"] = "info",
): LogLine[] {
  return [...lines, { text, kind }];
}

function App() {
  const [languages, setLanguages] = useState<LanguageInfo[]>([]);
  const [selectedLanguages, setSelectedLanguages] = useState<Set<string>>(
    new Set(),
  );
  const [xmlPath, setXmlPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [definitionsDir, setDefinitionsDir] = useState("");
  const [dialect, setDialect] = useState(DEFAULT_DIALECT);
  const [allDialects, setAllDialects] = useState(false);
  const [runtime, setRuntime] = useState(true);
  const [examples, setExamples] = useState(true);
  const [busy, setBusy] = useState(false);
  const [logLines, setLogLines] = useState<LogLine[]>([]);

  useEffect(() => {
    let cancelled = false;

    async function loadDefaults() {
      try {
        const [languageList, paths] = await Promise.all([
          listLanguages(),
          defaultPaths(),
        ]);

        if (cancelled) {
          return;
        }

        setLanguages(languageList);
        setSelectedLanguages(
          new Set(languageList.map((language) => language.id)),
        );
        setXmlPath(paths.default_xml);
        setOutputPath(paths.default_output);
        setDefinitionsDir(paths.definitions_dir);
      } catch (error) {
        if (!cancelled) {
          setLogLines((lines) =>
            appendLog(
              lines,
              `Failed to load defaults: ${String(error)}`,
              "error",
            ),
          );
        }
      }
    }

    void loadDefaults();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listenGenerateProgress((progress) => {
      setLogLines((lines) => appendLog(lines, progress.message));
    }).then((stop) => {
      unlisten = stop;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  function buildOptions(): GenerateOptions {
    return {
      inputs: xmlPath ? [xmlPath] : [],
      output: outputPath,
      languages: Array.from(selectedLanguages),
      dialect: dialect || null,
      all_dialects: allDialects,
      definitions_dir: definitionsDir,
      runtime,
      examples,
    };
  }

  async function handleBrowseXml() {
    const picked = await pickXmlFile();
    if (picked) {
      setXmlPath(picked);
    }
  }

  async function handleBrowseOutput() {
    const picked = await pickOutputDir();
    if (picked) {
      setOutputPath(picked);
    }
  }

  async function handleValidate() {
    if (!xmlPath) {
      setLogLines((lines) =>
        appendLog(lines, "Select an XML dialect file first.", "error"),
      );
      return;
    }

    setBusy(true);
    setLogLines((lines) => appendLog(lines, `Validating ${xmlPath}...`));

    try {
      const results = await validateDialects([xmlPath]);
      setLogLines((lines) =>
        results.reduce(
          (current, result) =>
            appendLog(current, formatValidateResult(result), "success"),
          lines,
        ),
      );
    } catch (error) {
      setLogLines((lines) =>
        appendLog(lines, String(error), "error"),
      );
    } finally {
      setBusy(false);
    }
  }

  async function handleGenerate() {
    if (selectedLanguages.size === 0) {
      setLogLines((lines) =>
        appendLog(lines, "Select at least one target language.", "error"),
      );
      return;
    }

    setBusy(true);
    setLogLines((lines) => appendLog(lines, "Generating..."));

    try {
      await generate(buildOptions());
      setLogLines((lines) =>
        appendLog(lines, "Generation finished.", "success"),
      );
    } catch (error) {
      setLogLines((lines) =>
        appendLog(lines, String(error), "error"),
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="app">
      <header className="app-header">
        <h1>MAVLink Generator</h1>
      </header>

      <section className="form-panel">
        <PathField
          label="XML"
          value={xmlPath}
          onChange={setXmlPath}
          onBrowse={() => void handleBrowseXml()}
          disabled={busy}
        />
        <PathField
          label="Output"
          value={outputPath}
          onChange={setOutputPath}
          onBrowse={() => void handleBrowseOutput()}
          disabled={busy}
        />

        <LanguageSelector
          languages={languages}
          selected={selectedLanguages}
          onChange={setSelectedLanguages}
          disabled={busy}
        />

        <OptionsPanel
          dialect={dialect}
          allDialects={allDialects}
          runtime={runtime}
          examples={examples}
          onDialectChange={setDialect}
          onAllDialectsChange={setAllDialects}
          onRuntimeChange={setRuntime}
          onExamplesChange={setExamples}
          disabled={busy}
        />

        <ActionBar
          busy={busy}
          onValidate={() => void handleValidate()}
          onGenerate={() => void handleGenerate()}
        />
      </section>

      <LogPanel lines={logLines} />
    </main>
  );
}

export default App;

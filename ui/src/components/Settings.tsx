import { useState } from "react";
import { useTauri } from "../hooks/useTauri";

export default function Settings() {
  const [owner, setOwner] = useState("");
  const [repo, setRepo] = useState("");
  const [importing, setImporting] = useState(false);
  const [importResult, setImportResult] = useState("");
  const tauri = useTauri();

  const handleImport = async () => {
    if (!owner.trim() || !repo.trim()) {
      setImportResult("Please enter both owner and repository name.");
      return;
    }

    setImporting(true);
    setImportResult("");
    try {
      const issues = await tauri.importGithubIssues(owner.trim(), repo.trim());
      setImportResult(`Successfully imported ${issues.length} issues from GitHub.`);
    } catch (err) {
      setImportResult(`Import failed: ${String(err)}`);
    } finally {
      setImporting(false);
    }
  };

  return (
    <div>
      <div className="settings-section">
        <h3>GitHub Import</h3>
        <p style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "16px" }}>
          Import issues from a public GitHub repository. They will be added to your local database.
        </p>
        <div className="settings-row">
          <input
            className="form-input"
            placeholder="Owner (e.g., facebook)"
            value={owner}
            onChange={(e) => setOwner(e.target.value)}
            style={{ maxWidth: "200px" }}
          />
          <span style={{ color: "var(--text-muted)" }}>/</span>
          <input
            className="form-input"
            placeholder="Repo (e.g., react)"
            value={repo}
            onChange={(e) => setRepo(e.target.value)}
            style={{ maxWidth: "200px" }}
          />
          <button
            className="btn btn-primary"
            onClick={handleImport}
            disabled={importing}
          >
            {importing ? "Importing..." : "Import Issues"}
          </button>
        </div>
        {importResult && (
          <div
            style={{
              marginTop: "12px",
              padding: "8px 12px",
              borderRadius: "6px",
              fontSize: "13px",
              background: importResult.startsWith("Successfully")
                ? "rgba(34,197,94,0.1)"
                : "rgba(248,81,73,0.1)",
              color: importResult.startsWith("Successfully")
                ? "var(--success)"
                : "var(--danger)",
              border: `1px solid ${importResult.startsWith("Successfully") ? "var(--success)" : "var(--danger)"}`,
            }}
          >
            {importResult}
          </div>
        )}
      </div>

      <div className="settings-section">
        <h3>About Tracera</h3>
        <p style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
          Tracera is a local-first project management desktop application built with
          Tauri 2 and React. Issues are stored in a local SQLite database.
        </p>
        <div style={{ marginTop: "12px", fontSize: "13px", color: "var(--text-muted)" }}>
          Version 0.1.0 (Milestone 1)
        </div>
      </div>
    </div>
  );
}

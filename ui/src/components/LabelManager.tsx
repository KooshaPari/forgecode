import { useState, useEffect, useCallback } from "react";
import { Label } from "../types";
import { useTauri } from "../hooks/useTauri";

const PRESET_COLORS = [
  "#58a6ff",
  "#3fb950",
  "#d29922",
  "#f85149",
  "#a371f7",
  "#79c0ff",
  "#56d364",
  "#e3b341",
  "#ff7b72",
  "#bc8cff",
  "#f0883e",
  "#8b949e",
];

export default function LabelManager() {
  const tauri = useTauri();
  const [labels, setLabels] = useState<Label[]>([]);
  const [loading, setLoading] = useState(true);
  const [newName, setNewName] = useState("");
  const [newColor, setNewColor] = useState(PRESET_COLORS[0]);
  const [creating, setCreating] = useState(false);
  const [result, setResult] = useState("");

  const loadLabels = useCallback(async () => {
    try {
      const list = await tauri.listLabels();
      setLabels(list);
    } catch (err) {
      console.error("Failed to load labels:", err);
    } finally {
      setLoading(false);
    }
  }, [tauri]);

  useEffect(() => {
    loadLabels();
  }, [loadLabels]);

  const handleCreate = async () => {
    if (!newName.trim()) {
      setResult("Please enter a label name.");
      return;
    }

    setCreating(true);
    setResult("");
    try {
      await tauri.createLabel({ name: newName.trim(), color: newColor });
      setResult("Label created!");
      setNewName("");
      await loadLabels();
    } catch (err) {
      setResult(`Failed: ${String(err)}`);
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await tauri.deleteLabel(id);
      await loadLabels();
    } catch (err) {
      console.error("Failed to delete label:", err);
    }
  };

  if (loading) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">...</div>
        <h3>Loading labels...</h3>
      </div>
    );
  }

  return (
    <div>
      <div className="settings-section">
        <h3>Create Label</h3>
        <div style={{ display: "flex", gap: "12px", alignItems: "flex-start", flexWrap: "wrap" }}>
          <div className="form-group" style={{ flex: 1, minWidth: "200px", marginBottom: 0 }}>
            <label className="form-label">Name</label>
            <input
              className="form-input"
              placeholder="e.g., bug, feature, docs"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleCreate()}
            />
          </div>
          <div className="form-group" style={{ marginBottom: 0 }}>
            <label className="form-label">Color</label>
            <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
              {PRESET_COLORS.map((c) => (
                <button
                  key={c}
                  onClick={() => setNewColor(c)}
                  style={{
                    width: "28px",
                    height: "28px",
                    borderRadius: "6px",
                    background: c,
                    border: newColor === c ? "2px solid #fff" : "2px solid transparent",
                    cursor: "pointer",
                    outline: newColor === c ? `2px solid ${c}` : "none",
                    outlineOffset: "2px",
                  }}
                />
              ))}
              <input
                type="color"
                value={newColor}
                onChange={(e) => setNewColor(e.target.value)}
                style={{
                  width: "28px",
                  height: "28px",
                  padding: 0,
                  border: "none",
                  cursor: "pointer",
                  borderRadius: "6px",
                }}
              />
            </div>
          </div>
          <button
            className="btn btn-primary"
            onClick={handleCreate}
            disabled={creating}
            style={{ marginTop: "22px" }}
          >
            {creating ? "..." : "Add"}
          </button>
        </div>
        {result && (
          <div
            style={{
              marginTop: "12px",
              padding: "8px 12px",
              borderRadius: "6px",
              fontSize: "13px",
              background: result.startsWith("Failed")
                ? "rgba(248,81,73,0.1)"
                : "rgba(34,197,94,0.1)",
              color: result.startsWith("Failed") ? "var(--danger)" : "var(--success)",
              border: `1px solid ${result.startsWith("Failed") ? "var(--danger)" : "var(--success)"}`,
            }}
          >
            {result}
          </div>
        )}
      </div>

      <div className="settings-section">
        <h3>Labels ({labels.length})</h3>
        {labels.length === 0 ? (
          <p style={{ fontSize: "13px", color: "var(--text-muted)" }}>
            No labels yet. Create one above.
          </p>
        ) : (
          <div className="label-list">
            {labels.map((label) => (
              <div key={label.id} className="label-list-item">
                <div className="label-list-info">
                  <span
                    className="label-color-swatch"
                    style={{ background: label.color }}
                  />
                  <span className="label-list-name">{label.name}</span>
                </div>
                <button
                  className="btn btn-danger btn-sm"
                  onClick={() => handleDelete(label.id)}
                >
                  Delete
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

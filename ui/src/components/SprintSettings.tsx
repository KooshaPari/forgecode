import { useState, useEffect, useCallback } from "react";
import { Sprint } from "../types";
import { useTauri } from "../hooks/useTauri";

export default function SprintSettings() {
  const tauri = useTauri();
  const [sprints, setSprints] = useState<Sprint[]>([]);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");
  const [goal, setGoal] = useState("");
  const [creating, setCreating] = useState(false);
  const [result, setResult] = useState("");

  const loadSprints = useCallback(async () => {
    try {
      const list = await tauri.listSprints();
      setSprints(list);
    } catch (err) {
      console.error("Failed to load sprints:", err);
    } finally {
      setLoading(false);
    }
  }, [tauri]);

  useEffect(() => {
    loadSprints();
  }, [loadSprints]);

  const handleCreate = async () => {
    if (!name.trim() || !startDate || !endDate) {
      setResult("Please fill in name, start date, and end date.");
      return;
    }

    setCreating(true);
    setResult("");
    try {
      await tauri.createSprint({
        name: name.trim(),
        start_date: startDate,
        end_date: endDate,
        goal: goal.trim(),
      });
      setResult("Sprint created successfully!");
      setName("");
      setStartDate("");
      setEndDate("");
      setGoal("");
      await loadSprints();
    } catch (err) {
      setResult(`Failed to create sprint: ${String(err)}`);
    } finally {
      setCreating(false);
    }
  };

  const handleActivate = async (sprintId: string) => {
    try {
      await tauri.activateSprint(sprintId);
      setResult("Sprint activated!");
      await loadSprints();
    } catch (err) {
      setResult(`Failed to activate: ${String(err)}`);
    }
  };

  const handleClose = async (sprintId: string) => {
    try {
      await tauri.closeSprint(sprintId);
      setResult("Sprint closed.");
      await loadSprints();
    } catch (err) {
      setResult(`Failed to close: ${String(err)}`);
    }
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
      });
    } catch {
      return dateStr;
    }
  };

  const statusLabel = (status: string) => {
    switch (status) {
      case "planning":
        return { text: "Planning", color: "#6366f1" };
      case "active":
        return { text: "Active", color: "#22c55e" };
      case "completed":
        return { text: "Completed", color: "#8b949e" };
      default:
        return { text: status, color: "#8b949e" };
    }
  };

  if (loading) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">...</div>
        <h3>Loading sprints...</h3>
      </div>
    );
  }

  return (
    <div>
      {/* Create Sprint Form */}
      <div className="settings-section">
        <h3>Create Sprint</h3>
        <div className="form-group">
          <label className="form-label">Sprint Name</label>
          <input
            className="form-input"
            placeholder="e.g., Sprint 1"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div className="form-row" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
          <div className="form-group">
            <label className="form-label">Start Date</label>
            <input
              className="form-input"
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label className="form-label">End Date</label>
            <input
              className="form-input"
              type="date"
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
            />
          </div>
        </div>
        <div className="form-group">
          <label className="form-label">Goal (optional)</label>
          <input
            className="form-input"
            placeholder="e.g., Ship the new dashboard"
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
          />
        </div>
        <button
          className="btn btn-primary"
          onClick={handleCreate}
          disabled={creating}
        >
          {creating ? "Creating..." : "Create Sprint"}
        </button>
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

      {/* Sprint List */}
      <div className="settings-section">
        <h3>Sprints</h3>
        {sprints.length === 0 ? (
          <p style={{ fontSize: "13px", color: "var(--text-muted)" }}>
            No sprints yet. Create your first sprint above.
          </p>
        ) : (
          <div className="sprint-list">
            {sprints.map((sprint) => {
              const st = statusLabel(sprint.status);
              return (
                <div key={sprint.id} className="sprint-list-item">
                  <div className="sprint-list-info">
                    <div className="sprint-list-name">
                      {sprint.name}
                      <span
                        className="sprint-status-badge"
                        style={{ color: st.color, marginLeft: "8px" }}
                      >
                        {st.text}
                      </span>
                    </div>
                    <div className="sprint-list-meta">
                      {formatDate(sprint.start_date)} - {formatDate(sprint.end_date)}
                      {sprint.goal && <span> | {sprint.goal}</span>}
                    </div>
                  </div>
                  <div className="sprint-list-actions">
                    {sprint.status === "planning" && (
                      <button
                        className="btn btn-primary btn-sm"
                        onClick={() => handleActivate(sprint.id)}
                      >
                        Activate
                      </button>
                    )}
                    {sprint.status === "active" && (
                      <button
                        className="btn btn-danger btn-sm"
                        onClick={() => handleClose(sprint.id)}
                      >
                        Close
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

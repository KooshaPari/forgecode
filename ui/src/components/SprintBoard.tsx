import { useState, useEffect, useCallback } from "react";
import {
  Issue,
  Sprint,
  SprintItem,
  SprintItemStatus,
  STATUS_COLORS,
} from "../types";
import { useTauri } from "../hooks/useTauri";
import BurndownChart from "./BurndownChart";
import VelocityChart from "./VelocityChart";

type SprintColumn = "todo" | "in_review" | "done";

const SPRINT_COLUMNS: { key: SprintColumn; label: string; color: string }[] = [
  { key: "todo", label: "To Do", color: "#6366f1" },
  { key: "in_review", label: "In Review", color: "#f59e0b" },
  { key: "done", label: "Done", color: "#22c55e" },
];

interface SprintBoardProps {
  issues: Issue[];
  onIssueMoved: (issueId: string, newStatus: string) => void;
}

export default function SprintBoard({ issues, onIssueMoved }: SprintBoardProps) {
  const tauri = useTauri();
  const [sprint, setSprint] = useState<Sprint | null>(null);
  const [sprintItems, setSprintItems] = useState<SprintItem[]>([]);
  const [velocity, setVelocity] = useState<
    { sprint_id: string; sprint_name: string; total_points: number; completed_points: number }[]
  >([]);
  const [loading, setLoading] = useState(true);
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [draggedType, setDraggedType] = useState<"backlog" | "sprint" | null>(null);
  const [dragOverZone, setDragOverZone] = useState<string | null>(null);
  const [pointInputs, setPointInputs] = useState<Record<string, string>>({});
  const [showBurndown, setShowBurndown] = useState(false);

  const loadData = useCallback(async () => {
    try {
      const activeSprint = await tauri.getActiveSprint();
      setSprint(activeSprint);
      if (activeSprint) {
        const items = await tauri.getSprintItems(activeSprint.id);
        setSprintItems(items);
      }
      const vel = await tauri.calculateVelocity(6);
      setVelocity(vel);
    } catch (err) {
      console.error("Failed to load sprint data:", err);
    } finally {
      setLoading(false);
    }
  }, [tauri]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Issues not in the current sprint
  const backlogIssues = sprint
    ? issues.filter((i) => !sprintItems.some((si) => si.issue_id === i.id))
    : issues;

  // Group sprint items by status
  const getColumnItems = (status: SprintColumn) =>
    sprintItems.filter((si) => si.status === status);

  // Calculate sprint progress
  const totalPoints = sprintItems.reduce((sum, si) => sum + si.points, 0);
  const completedPoints = sprintItems
    .filter((si) => si.status === "done")
    .reduce((sum, si) => sum + si.points, 0);
  const progressPercent = totalPoints > 0 ? Math.round((completedPoints / totalPoints) * 100) : 0;

  // Average velocity from completed sprints
  const avgVelocity =
    velocity.length > 0
      ? Math.round(
          velocity.reduce((sum, v) => sum + v.completed_points, 0) / velocity.length
        )
      : 0;

  // Sprint date formatting
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

  // Drag-and-drop handlers
  const handleBacklogDragStart = (e: React.DragEvent, issueId: string) => {
    setDraggedId(issueId);
    setDraggedType("backlog");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", `backlog:${issueId}`);
  };

  const handleSprintItemDragStart = (e: React.DragEvent, itemId: string) => {
    setDraggedId(itemId);
    setDraggedType("sprint");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", `sprint:${itemId}`);
  };

  const handleDragEnd = () => {
    setDraggedId(null);
    setDraggedType(null);
    setDragOverZone(null);
  };

  const handleDragOver = (e: React.DragEvent, zone: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setDragOverZone(zone);
  };

  const handleDragLeave = () => {
    setDragOverZone(null);
  };

  const handleDrop = async (e: React.DragEvent, targetStatus: string) => {
    e.preventDefault();
    const data = e.dataTransfer.getData("text/plain");
    if (!data || !sprint) return;

    const [type, id] = data.split(":");

    if (type === "backlog") {
      // Add issue from backlog to sprint
      const points = parseInt(pointInputs[id] || "1", 10) || 1;
      try {
        await tauri.addToSprint({ sprint_id: sprint.id, issue_id: id, points });
        await loadData();
      } catch (err) {
        console.error("Failed to add item to sprint:", err);
      }
    } else if (type === "sprint") {
      // Move sprint item to a new column
      try {
        await tauri.updateSprintItem({ item_id: id, status: targetStatus as SprintItemStatus });
        await loadData();
      } catch (err) {
        console.error("Failed to update sprint item:", err);
      }
    }

    setDraggedId(null);
    setDraggedType(null);
    setDragOverZone(null);
  };

  // Remove item from sprint (drag back to backlog)
  const handleRemoveFromSprint = async (itemId: string) => {
    try {
      await tauri.removeSprintItem(itemId);
      await loadData();
    } catch (err) {
      console.error("Failed to remove item from sprint:", err);
    }
  };

  // Backlog drop handler - remove from sprint
  const handleBacklogDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    const data = e.dataTransfer.getData("text/plain");
    if (!data) return;

    const [type, id] = data.split(":");
    if (type === "sprint") {
      await handleRemoveFromSprint(id);
    }

    setDraggedId(null);
    setDraggedType(null);
    setDragOverZone(null);
  };

  if (loading) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">...</div>
        <h3>Loading sprint...</h3>
      </div>
    );
  }

  if (!sprint) {
    return (
      <div className="empty-state">
        <div className="empty-state-icon">&#x1F4C5;</div>
        <h3>No Active Sprint</h3>
        <p>Create and activate a sprint in Sprint Settings to get started.</p>
      </div>
    );
  }

  return (
    <div className="sprint-board">
      {/* Sprint Header */}
      <div className="sprint-header">
        <div className="sprint-header-info">
          <h2>{sprint.name}</h2>
          <span className="sprint-status-badge active">Active</span>
        </div>
        <div className="sprint-header-meta">
          <span>
            {formatDate(sprint.start_date)} - {formatDate(sprint.end_date)}
          </span>
          {sprint.goal && <span className="sprint-goal">{sprint.goal}</span>}
        </div>
        <div className="sprint-progress">
          <div className="sprint-progress-bar">
            <div
              className="sprint-progress-fill"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          <span className="sprint-progress-text">
            {completedPoints}/{totalPoints} pts ({progressPercent}%)
          </span>
          <button
            className="btn btn-sm"
            onClick={() => setShowBurndown(!showBurndown)}
          >
            {showBurndown ? "Hide Charts" : "Charts"}
          </button>
        </div>
      </div>

      {/* Charts */}
      {showBurndown && (
        <div className="sprint-charts">
          <div className="sprint-chart-card">
            <BurndownChart sprintId={sprint.id} />
          </div>
          <div className="sprint-chart-card">
            <VelocityChart data={velocity} avgVelocity={avgVelocity} />
          </div>
        </div>
      )}

      {/* Main Content: Backlog + Sprint Columns */}
      <div className="sprint-content">
        {/* Backlog Sidebar */}
        <div
          className={`sprint-backlog ${dragOverZone === "backlog" ? "drag-over" : ""}`}
          onDragOver={(e) => handleDragOver(e, "backlog")}
          onDragLeave={handleDragLeave}
          onDrop={handleBacklogDrop}
        >
          <div className="sprint-backlog-header">
            <h3>Backlog</h3>
            <span className="sprint-column-count">{backlogIssues.length}</span>
          </div>
          <div className="sprint-backlog-items">
            {backlogIssues.length === 0 && (
              <div className="sprint-empty-text">All issues in sprint</div>
            )}
            {backlogIssues.map((issue) => (
              <div
                key={issue.id}
                className={`issue-card ${draggedId === issue.id ? "dragging" : ""}`}
                draggable
                onDragStart={(e) => handleBacklogDragStart(e, issue.id)}
                onDragEnd={handleDragEnd}
              >
                <div className="issue-card-title">{issue.title}</div>
                <div className="issue-card-footer">
                  <div className="sprint-points-input">
                    <label>pts</label>
                    <input
                      type="number"
                      min={1}
                      max={21}
                      value={pointInputs[issue.id] || "1"}
                      onChange={(e) =>
                        setPointInputs((prev) => ({
                          ...prev,
                          [issue.id]: e.target.value,
                        }))
                      }
                      onClick={(e) => e.stopPropagation()}
                    />
                  </div>
                  <span className={`priority-badge ${issue.priority}`}>
                    {issue.priority}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Sprint Columns */}
        <div className="sprint-columns">
          {SPRINT_COLUMNS.map((col) => {
            const items = getColumnItems(col.key);
            const isOver = dragOverZone === col.key;

            return (
              <div key={col.key} className="board-column">
                <div className="board-column-header">
                  <div className="board-column-title">
                    <span
                      style={{
                        width: "10px",
                        height: "10px",
                        borderRadius: "50%",
                        background: col.color,
                        display: "inline-block",
                      }}
                    />
                    {col.label}
                  </div>
                  <span className="board-column-count">{items.length}</span>
                </div>
                <div
                  className={`board-column-body ${isOver ? "drag-over" : ""}`}
                  onDragOver={(e) => handleDragOver(e, col.key)}
                  onDragLeave={handleDragLeave}
                  onDrop={(e) => handleDrop(e, col.key)}
                >
                  {items.length === 0 && !isOver && (
                    <div
                      style={{
                        padding: "24px",
                        textAlign: "center",
                        color: "var(--text-muted)",
                        fontSize: "13px",
                      }}
                    >
                      Drag issues here
                    </div>
                  )}
                  {items.map((sprintItem) => {
                    const issue = issues.find((i) => i.id === sprintItem.issue_id);
                    if (!issue) return null;
                    return (
                      <div
                        key={sprintItem.id}
                        className={`issue-card ${draggedId === sprintItem.id ? "dragging" : ""}`}
                        draggable
                        onDragStart={(e) =>
                          handleSprintItemDragStart(e, sprintItem.id)
                        }
                        onDragEnd={handleDragEnd}
                      >
                        <div className="issue-card-title">{issue.title}</div>
                        <div className="issue-card-footer">
                          <span className="sprint-item-points">
                            {sprintItem.points} pts
                          </span>
                          <span className={`priority-badge ${issue.priority}`}>
                            {issue.priority}
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Velocity Display */}
      {velocity.length > 0 && (
        <div className="sprint-velocity-summary">
          <span className="sprint-velocity-label">Avg Velocity:</span>
          <span className="sprint-velocity-value">{avgVelocity} pts/sprint</span>
          <span className="sprint-velocity-rate">
            (
            {velocity.length > 0
              ? Math.round(
                  (velocity.reduce((s, v) => s + v.completed_points, 0) /
                    velocity.reduce((s, v) => s + v.total_points, 0)) *
                    100
                )
              : 0}
            % completion rate)
          </span>
        </div>
      )}
    </div>
  );
}

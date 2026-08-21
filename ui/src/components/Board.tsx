import { useState } from "react";
import { Issue, Status, STATUS_COLORS } from "../types";

interface BoardProps {
  issues: Issue[];
  onIssueMoved: (issueId: string, newStatus: string) => void;
  onSelectIssue: (issue: Issue) => void;
}

const COLUMNS: Status[] = ["Backlog", "In Progress", "Done"];

export default function Board({ issues, onIssueMoved, onSelectIssue }: BoardProps) {
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragOverCol, setDragOverCol] = useState<string | null>(null);

  const handleDragStart = (e: React.DragEvent, issueId: string) => {
    setDraggedId(issueId);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", issueId);
  };

  const handleDragEnd = () => {
    setDraggedId(null);
    setDragOverCol(null);
  };

  const handleDragOver = (e: React.DragEvent, status: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setDragOverCol(status);
  };

  const handleDragLeave = () => {
    setDragOverCol(null);
  };

  const handleDrop = (e: React.DragEvent, status: string) => {
    e.preventDefault();
    const issueId = e.dataTransfer.getData("text/plain");
    if (issueId) {
      onIssueMoved(issueId, status);
    }
    setDraggedId(null);
    setDragOverCol(null);
  };

  return (
    <div className="board">
      {COLUMNS.map((status) => {
        const columnIssues = issues.filter((i) => i.status === status);
        const isOver = dragOverCol === status;

        return (
          <div key={status} className="board-column">
            <div className="board-column-header">
              <div className="board-column-title">
                <span
                  style={{
                    width: "10px",
                    height: "10px",
                    borderRadius: "50%",
                    background: STATUS_COLORS[status],
                    display: "inline-block",
                  }}
                />
                {status}
              </div>
              <span className="board-column-count">{columnIssues.length}</span>
            </div>
            <div
              className={`board-column-body ${isOver ? "drag-over" : ""}`}
              onDragOver={(e) => handleDragOver(e, status)}
              onDragLeave={handleDragLeave}
              onDrop={(e) => handleDrop(e, status)}
            >
              {columnIssues.length === 0 && !isOver && (
                <div
                  style={{
                    padding: "24px",
                    textAlign: "center",
                    color: "var(--text-muted)",
                    fontSize: "13px",
                  }}
                >
                  No issues
                </div>
              )}
              {columnIssues.map((issue) => (
                <div
                  key={issue.id}
                  className={`issue-card ${draggedId === issue.id ? "dragging" : ""}`}
                  draggable
                  onDragStart={(e) => handleDragStart(e, issue.id)}
                  onDragEnd={handleDragEnd}
                  onClick={() => onSelectIssue(issue)}
                >
                  <div className="issue-card-title">{issue.title}</div>
                  <div className="issue-card-footer">
                    <div className="issue-card-labels">
                      {issue.labels &&
                        issue.labels
                          .split(",")
                          .filter((l) => l.trim())
                          .slice(0, 3)
                          .map((label, idx) => (
                            <span key={idx} className="issue-card-label">
                              {label.trim()}
                            </span>
                          ))}
                    </div>
                    <span className={`priority-badge ${issue.priority}`}>
                      {issue.priority}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

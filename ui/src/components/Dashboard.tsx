import { Issue, View, STATUS_COLORS, PRIORITY_COLORS } from "../types";

interface DashboardProps {
  issues: Issue[];
  onNavigate: (view: View) => void;
  onSelectIssue: (issue: Issue) => void;
}

export default function Dashboard({ issues, onNavigate, onSelectIssue }: DashboardProps) {
  const statusCounts = {
    Backlog: issues.filter((i) => i.status === "Backlog").length,
    "In Progress": issues.filter((i) => i.status === "In Progress").length,
    Done: issues.filter((i) => i.status === "Done").length,
  };

  const priorityCounts = {
    Critical: issues.filter((i) => i.priority === "Critical").length,
    High: issues.filter((i) => i.priority === "High").length,
    Medium: issues.filter((i) => i.priority === "Medium").length,
    Low: issues.filter((i) => i.priority === "Low").length,
  };

  const recentIssues = [...issues]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 8);

  return (
    <div>
      <div className="dashboard-stats">
        <div className="stat-card">
          <div className="stat-card-label">Total Issues</div>
          <div className="stat-card-value">{issues.length}</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-label">Backlog</div>
          <div className="stat-card-value" style={{ color: STATUS_COLORS.Backlog }}>
            {statusCounts.Backlog}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-label">In Progress</div>
          <div className="stat-card-value" style={{ color: STATUS_COLORS["In Progress"] }}>
            {statusCounts["In Progress"]}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-label">Done</div>
          <div className="stat-card-value" style={{ color: STATUS_COLORS.Done }}>
            {statusCounts.Done}
          </div>
        </div>
      </div>

      <div className="dashboard-stats">
        <div className="stat-card">
          <div className="stat-card-label">Critical</div>
          <div className="stat-card-value" style={{ color: PRIORITY_COLORS.Critical }}>
            {priorityCounts.Critical}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-label">High</div>
          <div className="stat-card-value" style={{ color: PRIORITY_COLORS.High }}>
            {priorityCounts.High}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-label">Medium</div>
          <div className="stat-card-value" style={{ color: PRIORITY_COLORS.Medium }}>
            {priorityCounts.Medium}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-card-label">Low</div>
          <div className="stat-card-value" style={{ color: PRIORITY_COLORS.Low }}>
            {priorityCounts.Low}
          </div>
        </div>
      </div>

      <div className="dashboard-section">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
          <h3>Recent Activity</h3>
          <button className="btn btn-sm" onClick={() => onNavigate("board")}>
            View Board
          </button>
        </div>

        {recentIssues.length === 0 ? (
          <div className="empty-state" style={{ padding: "32px" }}>
            <div className="empty-state-icon">+</div>
            <h3>No issues yet</h3>
            <p>Create your first issue to get started.</p>
          </div>
        ) : (
          <div className="recent-list">
            {recentIssues.map((issue) => (
              <div
                key={issue.id}
                className="recent-item"
                onClick={() => onSelectIssue(issue)}
              >
                <div>
                  <div className="recent-item-title">{issue.title}</div>
                  <div className="recent-item-meta">
                    Updated {new Date(issue.updated_at).toLocaleDateString()}
                  </div>
                </div>
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  <span className={`priority-badge ${issue.priority}`}>
                    {issue.priority}
                  </span>
                  <span className="status-badge">{issue.status}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

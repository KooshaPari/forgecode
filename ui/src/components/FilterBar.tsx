import { useState } from "react";
import {
  FilterState,
  STATUS_OPTIONS,
  PRIORITY_OPTIONS,
  Sprint,
  Label,
} from "../types";

interface FilterBarProps {
  filters: FilterState;
  onChange: (filters: FilterState) => void;
  assignees: string[];
  sprints: Sprint[];
  labels: Label[];
}

export default function FilterBar({
  filters,
  onChange,
  assignees,
  sprints,
  labels,
}: FilterBarProps) {
  const [expanded, setExpanded] = useState(false);

  const updateFilter = <K extends keyof FilterState>(
    key: K,
    value: FilterState[K]
  ) => {
    onChange({ ...filters, [key]: value });
  };

  const hasActiveFilters =
    filters.search ||
    filters.status ||
    filters.priority ||
    filters.assignee ||
    filters.labelIds.length > 0 ||
    filters.sprintId;

  const clearFilters = () => {
    onChange({
      search: "",
      status: "",
      priority: "",
      assignee: "",
      labelIds: [],
      sprintId: "",
    });
  };

  const toggleLabel = (labelId: string) => {
    const current = filters.labelIds;
    if (current.includes(labelId)) {
      updateFilter(
        "labelIds",
        current.filter((id) => id !== labelId)
      );
    } else {
      updateFilter("labelIds", [...current, labelId]);
    }
  };

  return (
    <div className="filter-bar">
      {/* Primary row: search + status + priority */}
      <div className="filter-bar-row">
        <div className="filter-search">
          <input
            className="form-input"
            placeholder="Search issues..."
            value={filters.search}
            onChange={(e) => updateFilter("search", e.target.value)}
          />
        </div>
        <select
          className="form-select"
          value={filters.status}
          onChange={(e) => updateFilter("status", e.target.value)}
        >
          <option value="">All Statuses</option>
          {STATUS_OPTIONS.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <select
          className="form-select"
          value={filters.priority}
          onChange={(e) => updateFilter("priority", e.target.value)}
        >
          <option value="">All Priorities</option>
          {PRIORITY_OPTIONS.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <button
          className={`btn btn-sm ${expanded ? "btn-active" : ""}`}
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? "Less" : "More Filters"}
        </button>
        {hasActiveFilters && (
          <button className="btn btn-sm btn-danger" onClick={clearFilters}>
            Clear
          </button>
        )}
      </div>

      {/* Expanded row: assignee, label, sprint */}
      {expanded && (
        <div className="filter-bar-row" style={{ marginTop: "8px" }}>
          <select
            className="form-select"
            value={filters.assignee}
            onChange={(e) => updateFilter("assignee", e.target.value)}
          >
            <option value="">All Assignees</option>
            {assignees.map((a) => (
              <option key={a} value={a}>
                {a}
              </option>
            ))}
          </select>
          <select
            className="form-select"
            value={filters.sprintId}
            onChange={(e) => updateFilter("sprintId", e.target.value)}
          >
            <option value="">All Sprints</option>
            {sprints.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          <div className="filter-label-select">
            <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
              Labels:
            </span>
            <div style={{ display: "flex", gap: "4px", flexWrap: "wrap" }}>
              {labels.map((label) => (
                <button
                  key={label.id}
                  className={`filter-label-chip ${filters.labelIds.includes(label.id) ? "active" : ""}`}
                  onClick={() => toggleLabel(label.id)}
                  style={{
                    borderColor: filters.labelIds.includes(label.id)
                      ? label.color
                      : undefined,
                  }}
                >
                  <span
                    style={{
                      width: "8px",
                      height: "8px",
                      borderRadius: "50%",
                      background: label.color,
                      display: "inline-block",
                    }}
                  />
                  {label.name}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

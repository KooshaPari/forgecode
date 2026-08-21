export type Priority = "Critical" | "High" | "Medium" | "Low";
export type Status = "Backlog" | "In Progress" | "Done";

export interface Issue {
  id: string;
  title: string;
  description: string;
  status: Status;
  priority: Priority;
  assignee: string;
  labels: string;
  created_at: string;
  updated_at: string;
}

export interface CreateIssueRequest {
  title: string;
  description?: string;
  status?: Status;
  priority?: Priority;
  assignee?: string;
  labels?: string;
}

export interface UpdateIssueRequest {
  id: string;
  title?: string;
  description?: string;
  status?: Status;
  priority?: Priority;
  assignee?: string;
  labels?: string;
}

export type View =
  | "dashboard"
  | "board"
  | "issues"
  | "settings";

export const STATUS_OPTIONS: Status[] = ["Backlog", "In Progress", "Done"];
export const PRIORITY_OPTIONS: Priority[] = ["Critical", "High", "Medium", "Low"];

export const PRIORITY_COLORS: Record<Priority, string> = {
  Critical: "#ef4444",
  High: "#f97316",
  Medium: "#eab308",
  Low: "#22c55e",
};

export const STATUS_COLORS: Record<Status, string> = {
  Backlog: "#6366f1",
  "In Progress": "#f59e0b",
  Done: "#22c55e",
};

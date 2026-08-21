export type Priority = "Critical" | "High" | "Medium" | "Low";
export type Status = "Backlog" | "In Progress" | "Done";
export type SprintStatus = "planning" | "active" | "completed";
export type SprintItemStatus = "todo" | "in_review" | "done";

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

export interface Sprint {
  id: string;
  name: string;
  start_date: string;
  end_date: string;
  status: SprintStatus;
  goal: string;
  created_at: string;
}

export interface CreateSprintRequest {
  name: string;
  start_date: string;
  end_date: string;
  goal?: string;
}

export interface SprintItem {
  id: string;
  sprint_id: string;
  issue_id: string;
  status: SprintItemStatus;
  points: number;
  created_at: string;
}

export interface AddToSprintRequest {
  sprint_id: string;
  issue_id: string;
  points?: number;
}

export interface UpdateSprintItemRequest {
  item_id: string;
  status: SprintItemStatus;
}

export interface VelocityData {
  sprint_id: string;
  sprint_name: string;
  total_points: number;
  completed_points: number;
}

export interface BurndownPoint {
  date: string;
  remaining: number;
}

export interface BurndownData {
  total_points: number;
  points: BurndownPoint[];
}

export interface Label {
  id: string;
  name: string;
  color: string;
  created_at: string;
}

export interface CreateLabelRequest {
  name: string;
  color?: string;
}

export interface FilterState {
  search: string;
  status: string;
  priority: string;
  assignee: string;
  labelIds: string[];
  sprintId: string;
}

export type View =
  | "dashboard"
  | "board"
  | "issues"
  | "sprint"
  | "sprint-settings"
  | "labels"
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

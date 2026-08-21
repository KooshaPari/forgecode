export type IssueStatus = 'backlog' | 'in-progress' | 'done';
export type IssuePriority = 'critical' | 'high' | 'medium' | 'low';

export interface Issue {
  id: string;
  title: string;
  description: string;
  status: IssueStatus;
  priority: IssuePriority;
  assignee: string;
  labels: string[];
  created_at: string;
  updated_at: string;
  sprint_id: string | null;
}

export interface Sprint {
  id: string;
  name: string;
  start_date: string;
  end_date: string;
  goal: string;
  active: boolean;
}

export interface BoardStats {
  total: number;
  backlog: number;
  in_progress: number;
  done: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
}

export interface SprintProgress {
  sprint: Sprint;
  total_issues: number;
  completed_issues: number;
  velocity: number;
}

export interface CreateIssuePayload {
  title: string;
  description: string;
  status: string;
  priority: string;
  assignee: string;
  labels: string[];
  sprint_id: string | null;
}

export interface UpdateIssuePayload {
  id: string;
  title?: string;
  description?: string;
  status?: string;
  priority?: string;
  assignee?: string;
  labels?: string[];
  sprint_id?: string | null;
}

export type ViewType = 'dashboard' | 'board' | 'sprints' | 'settings';

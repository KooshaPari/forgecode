import { invoke } from '@tauri-apps/api/core';
import type { Issue, BoardStats, SprintProgress, CreateIssuePayload, UpdateIssuePayload, Sprint } from './types';

export const api = {
  async getIssues(): Promise<Issue[]> {
    return invoke<Issue[]>('get_issues');
  },

  async createIssue(payload: CreateIssuePayload): Promise<Issue> {
    return invoke<Issue>('create_issue', { payload });
  },

  async updateIssue(payload: UpdateIssuePayload): Promise<Issue> {
    return invoke<Issue>('update_issue', { payload });
  },

  async deleteIssue(id: string): Promise<void> {
    return invoke<void>('delete_issue', { id });
  },

  async getBoardStats(): Promise<BoardStats> {
    return invoke<BoardStats>('get_board_stats');
  },

  async getSprintProgress(): Promise<SprintProgress[]> {
    return invoke<SprintProgress[]>('get_sprint_progress');
  },

  async createSprint(payload: { name: string; start_date: string; end_date: string; goal: string }): Promise<Sprint> {
    return invoke<Sprint>('create_sprint', { payload });
  },

  async activateSprint(id: string): Promise<Sprint> {
    return invoke<Sprint>('activate_sprint', { id });
  },
};

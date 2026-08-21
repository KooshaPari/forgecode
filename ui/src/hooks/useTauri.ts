import { invoke } from "@tauri-apps/api/core";
import {
  Issue,
  CreateIssueRequest,
  UpdateIssueRequest,
} from "../types";

/**
 * Custom hook wrapping Tauri IPC invoke calls.
 * Returns a stable set of action functions that map 1:1 to backend commands.
 */
export function useTauri() {
  const createIssue = async (request: CreateIssueRequest): Promise<Issue> => {
    return invoke<Issue>("create_issue", { request });
  };

  const listIssues = async (): Promise<Issue[]> => {
    return invoke<Issue[]>("list_issues");
  };

  const updateIssue = async (request: UpdateIssueRequest): Promise<Issue> => {
    return invoke<Issue>("update_issue", { request });
  };

  const deleteIssue = async (id: string): Promise<boolean> => {
    return invoke<boolean>("delete_issue", { id });
  };

  const importGithubIssues = async (
    owner: string,
    repo: string
  ): Promise<Issue[]> => {
    return invoke<Issue[]>("import_github_issues", { owner, repo });
  };

  return {
    createIssue,
    listIssues,
    updateIssue,
    deleteIssue,
    importGithubIssues,
  };
}

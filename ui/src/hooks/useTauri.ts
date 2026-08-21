import { invoke } from "@tauri-apps/api/core";
import {
  Issue,
  CreateIssueRequest,
  UpdateIssueRequest,
  Sprint,
  CreateSprintRequest,
  SprintItem,
  AddToSprintRequest,
  UpdateSprintItemRequest,
  VelocityData,
  BurndownData,
  Label,
  CreateLabelRequest,
} from "../types";

/**
 * Custom hook wrapping Tauri IPC invoke calls.
 * Returns a stable set of action functions that map 1:1 to backend commands.
 */
export function useTauri() {
  // ── Issue commands ────────────────────────────────────────────

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

  // ── Sprint commands ───────────────────────────────────────────

  const createSprint = async (request: CreateSprintRequest): Promise<Sprint> => {
    return invoke<Sprint>("create_sprint", { request });
  };

  const listSprints = async (): Promise<Sprint[]> => {
    return invoke<Sprint[]>("list_sprints");
  };

  const getActiveSprint = async (): Promise<Sprint | null> => {
    return invoke<Sprint | null>("get_active_sprint");
  };

  const activateSprint = async (sprintId: string): Promise<void> => {
    return invoke<void>("activate_sprint", { sprintId });
  };

  const closeSprint = async (sprintId: string): Promise<void> => {
    return invoke<void>("close_sprint", { sprintId });
  };

  const addToSprint = async (request: AddToSprintRequest): Promise<SprintItem> => {
    return invoke<SprintItem>("add_to_sprint", { request });
  };

  const removeSprintItem = async (itemId: string): Promise<boolean> => {
    return invoke<boolean>("remove_sprint_item", { itemId });
  };

  const updateSprintItem = async (request: UpdateSprintItemRequest): Promise<SprintItem> => {
    return invoke<SprintItem>("update_sprint_item", { request });
  };

  const getSprintItems = async (sprintId: string): Promise<SprintItem[]> => {
    return invoke<SprintItem[]>("get_sprint_items", { sprintId });
  };

  const calculateVelocity = async (numSprints: number): Promise<VelocityData[]> => {
    return invoke<VelocityData[]>("calculate_velocity", { numSprints });
  };

  const getSprintBurndown = async (sprintId: string): Promise<BurndownData> => {
    return invoke<BurndownData>("get_sprint_burndown", { sprintId });
  };

  // ── Label commands ────────────────────────────────────────────

  const createLabel = async (request: CreateLabelRequest): Promise<Label> => {
    return invoke<Label>("create_label", { request });
  };

  const listLabels = async (): Promise<Label[]> => {
    return invoke<Label[]>("list_labels");
  };

  const deleteLabel = async (id: string): Promise<boolean> => {
    return invoke<boolean>("delete_label", { id });
  };

  return {
    // Issues
    createIssue,
    listIssues,
    updateIssue,
    deleteIssue,
    importGithubIssues,
    // Sprints
    createSprint,
    listSprints,
    getActiveSprint,
    activateSprint,
    closeSprint,
    addToSprint,
    removeSprintItem,
    updateSprintItem,
    getSprintItems,
    calculateVelocity,
    getSprintBurndown,
    // Labels
    createLabel,
    listLabels,
    deleteLabel,
  };
}

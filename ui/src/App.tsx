import { useState, useEffect, useCallback } from "react";
import { Issue, View } from "./types";
import { useTauri } from "./hooks/useTauri";
import Sidebar from "./components/Sidebar";
import Dashboard from "./components/Dashboard";
import Board from "./components/Board";
import IssueList from "./components/IssueList";
import IssueDetail from "./components/IssueDetail";
import CreateIssue from "./components/CreateIssue";
import Settings from "./components/Settings";
import SprintBoard from "./components/SprintBoard";
import SprintSettings from "./components/SprintSettings";
import LabelManager from "./components/LabelManager";

export default function App() {
  const [view, setView] = useState<View>("dashboard");
  const [issues, setIssues] = useState<Issue[]>([]);
  const [selectedIssue, setSelectedIssue] = useState<Issue | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [loading, setLoading] = useState(true);
  const tauri = useTauri();

  const refreshIssues = useCallback(async () => {
    try {
      const list = await tauri.listIssues();
      setIssues(list);
    } catch (err) {
      console.error("Failed to load issues:", err);
    } finally {
      setLoading(false);
    }
  }, [tauri]);

  useEffect(() => {
    refreshIssues();
  }, [refreshIssues]);

  const handleIssueCreated = () => {
    setShowCreateModal(false);
    refreshIssues();
  };

  const handleIssueUpdated = () => {
    setSelectedIssue(null);
    refreshIssues();
  };

  const handleIssueDeleted = () => {
    setSelectedIssue(null);
    refreshIssues();
  };

  const handleIssueMoved = (issueId: string, newStatus: string) => {
    setIssues((prev) =>
      prev.map((i) =>
        i.id === issueId ? { ...i, status: newStatus as Issue["status"] } : i
      )
    );
    // Persist in background
    tauri
      .updateIssue({ id: issueId, status: newStatus as Issue["status"] })
      .catch(console.error);
  };

  const renderView = () => {
    if (loading) {
      return (
        <div className="empty-state">
          <div className="empty-state-icon">...</div>
          <h3>Loading issues...</h3>
        </div>
      );
    }

    switch (view) {
      case "dashboard":
        return (
          <Dashboard
            issues={issues}
            onNavigate={setView}
            onSelectIssue={setSelectedIssue}
          />
        );
      case "board":
        return (
          <Board
            issues={issues}
            onIssueMoved={handleIssueMoved}
            onSelectIssue={setSelectedIssue}
          />
        );
      case "issues":
        return (
          <IssueList
            issues={issues}
            onSelectIssue={setSelectedIssue}
            onRefresh={refreshIssues}
          />
        );
      case "sprint":
        return (
          <SprintBoard
            issues={issues}
            onIssueMoved={handleIssueMoved}
          />
        );
      case "sprint-settings":
        return <SprintSettings />;
      case "labels":
        return <LabelManager />;
      case "settings":
        return <Settings />;
      default:
        return null;
    }
  };

  const viewTitles: Record<View, string> = {
    dashboard: "Dashboard",
    board: "Board",
    issues: "Issues",
    sprint: "Sprint Board",
    "sprint-settings": "Sprint Settings",
    labels: "Labels",
    settings: "Settings",
  };

  return (
    <div className="app-layout">
      <Sidebar currentView={view} onNavigate={setView} />
      <div className="app-main">
        <div className="app-header">
          <h1>{viewTitles[view]}</h1>
          <button
            className="btn btn-primary"
            onClick={() => setShowCreateModal(true)}
          >
            + New Issue
          </button>
        </div>
        <div className="app-content">{renderView()}</div>
      </div>

      {/* Modals */}
      {showCreateModal && (
        <CreateIssue
          onCreated={handleIssueCreated}
          onClose={() => setShowCreateModal(false)}
        />
      )}

      {selectedIssue && (
        <IssueDetail
          issue={selectedIssue}
          onUpdated={handleIssueUpdated}
          onDeleted={handleIssueDeleted}
          onClose={() => setSelectedIssue(null)}
        />
      )}
    </div>
  );
}

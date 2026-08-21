import { View } from "../types";

interface SidebarProps {
  currentView: View;
  onNavigate: (view: View) => void;
}

const navItems: { id: View; label: string; icon: string }[] = [
  { id: "dashboard", label: "Dashboard", icon: "\u25A0" },
  { id: "board", label: "Board", icon: "\u2637" },
  { id: "issues", label: "Issues", icon: "\u25CF" },
  { id: "sprint", label: "Sprint Board", icon: "\u25B6" },
  { id: "sprint-settings", label: "Sprint Settings", icon: "\u2699" },
  { id: "labels", label: "Labels", icon: "\u25C6" },
  { id: "settings", label: "Settings", icon: "\u2699" },
];

export default function Sidebar({ currentView, onNavigate }: SidebarProps) {
  return (
    <div className="sidebar">
      <div className="sidebar-brand">
        <h2>Tracera</h2>
        <span>Project Management</span>
      </div>
      <nav className="sidebar-nav">
        {navItems.map((item) => (
          <button
            key={item.id}
            className={`sidebar-nav-item ${currentView === item.id ? "active" : ""}`}
            onClick={() => onNavigate(item.id)}
          >
            <span style={{ fontSize: "16px", width: "20px", textAlign: "center" }}>
              {item.icon}
            </span>
            {item.label}
          </button>
        ))}
      </nav>
      <div className="sidebar-footer">Tracera v0.1.0</div>
    </div>
  );
}

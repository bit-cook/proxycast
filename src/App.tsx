import { useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./components/Dashboard";
import { Providers } from "./components/Providers";
import { SettingsPage } from "./components/settings";
import { SwitchPage } from "./components/switch";
import { ClientsPage } from "./components/clients";
import { McpPage } from "./components/mcp";
import { PromptsPage } from "./components/prompts";
import { ApiServerPage } from "./components/api-server/ApiServerPage";
import { SkillsPage } from "./components/skills";
import { ProviderPoolPage } from "./components/provider-pool";
import { RoutingPage } from "./components/routing";
import { ResiliencePage } from "./components/resilience";
import { ConfigPage } from "./components/config";
import { MonitoringPage } from "./components/monitoring";

type Page =
  | "dashboard"
  | "clients"
  | "api-server"
  | "providers"
  | "settings"
  | "switch"
  | "mcp"
  | "prompts"
  | "skills"
  | "provider-pool"
  | "routing"
  | "resilience"
  | "config"
  | "monitoring";

function App() {
  const [currentPage, setCurrentPage] = useState<Page>("dashboard");

  const renderPage = () => {
    switch (currentPage) {
      case "dashboard":
        return <Dashboard />;
      case "provider-pool":
        return <ProviderPoolPage />;
      case "clients":
        return <ClientsPage />;
      case "api-server":
        return <ApiServerPage />;
      case "providers":
        return <Providers />;
      case "settings":
        return <SettingsPage />;
      case "switch":
        return <SwitchPage />;
      case "mcp":
        return <McpPage />;
      case "prompts":
        return <PromptsPage />;
      case "skills":
        return <SkillsPage />;
      case "routing":
        return <RoutingPage />;
      case "resilience":
        return <ResiliencePage />;
      case "config":
        return <ConfigPage />;
      case "monitoring":
        return <MonitoringPage />;
      default:
        return <Dashboard />;
    }
  };

  return (
    <div className="flex h-screen bg-background">
      <Sidebar currentPage={currentPage} onNavigate={setCurrentPage} />
      <main className="flex-1 overflow-auto p-6">{renderPage()}</main>
    </div>
  );
}

export default App;

/**
 * 设置页面主布局组件
 *
 * 采用左侧边栏 + 右侧内容的布局
 * 参考成熟产品的设置布局设计
 */

import { useState, ReactNode, useEffect } from "react";
import styled from "styled-components";
import { Home } from "lucide-react";
import { SettingsSidebar } from "./SettingsSidebar";
import { SettingsTabs } from "@/types/settings";
import { buildHomeAgentParams } from "@/lib/workspace/navigation";
import { Page, PageParams } from "@/types/page";

// 外观设置（迁移自原 GeneralSettings）
import { GeneralSettings } from "../../settings/GeneralSettings";
// 网络代理
import { ProxySettings } from "../../settings/ProxySettings";
// 数据存储
import { DirectorySettings } from "../../settings/DirectorySettings";
import { QuotaSettings } from "../../settings/QuotaSettings";
// 安全设置
import { TlsSettings } from "../../settings/TlsSettings";
import { RemoteManagementSettings } from "../../settings/RemoteManagementSettings";
// 外部工具
import { ExternalToolsSettings } from "../../settings/ExternalToolsSettings";
// 实验功能
import { ExperimentalSettings } from "../../settings/ExperimentalSettings";
// 开发者
import { DeveloperSettings } from "../../settings/DeveloperSettings";
// 关于
import { AboutSection } from "../../settings/AboutSection";
// 扩展设置
import { ExtensionsSettings } from "../../settings/ExtensionsSettings";
// 快捷键设置
import { HotkeysSettings } from "../general/hotkeys";
// 聊天外观设置
import { ChatAppearanceSettings } from "../general/chat-appearance";
// 记忆设置
// 语音服务设置
import { VoiceSettings } from "../agent/voice";
// 助理服务设置
import { AssistantSettings } from "../agent/assistant";
// 图像生成设置
import { ImageGenSettings } from "../agent/image-gen";
// 数据统计
import { StatsSettings } from "../account/stats";
// 个人资料
import { ProfileSettings } from "../account/profile";
import { ProviderPoolPage } from "@/components/provider-pool";
import { ApiServerPage } from "@/components/api-server/ApiServerPage";
import { McpPanel } from "@/components/mcp";

import { SettingHeader } from "../features/SettingHeader";

const LayoutContainer = styled.div`
  display: flex;
  flex: 1;
  min-height: 0;
  background: hsl(var(--background));
`;

const ContentContainer = styled.main`
  flex: 1;
  overflow-y: auto;
  padding: 24px 32px;

  &::-webkit-scrollbar {
    width: 6px;
  }

  &::-webkit-scrollbar-track {
    background: transparent;
  }

  &::-webkit-scrollbar-thumb {
    background: hsl(var(--border));
    border-radius: 3px;
  }
`;

const ContentWrapper = styled.div<{ $wide: boolean }>`
  width: 100%;
  max-width: ${({ $wide }) => ($wide ? "none" : "800px")};
`;

const HeaderBar = styled.div`
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 24px;
  border-bottom: 1px solid hsl(var(--border));
  background: hsl(var(--background));
`;

const BackButton = styled.button`
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: 8px;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--foreground));
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: hsl(var(--accent));
    border-color: hsl(var(--accent));
  }

  svg {
    width: 16px;
    height: 16px;
  }
`;

const PlaceholderPage = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  color: hsl(var(--muted-foreground));
  text-align: center;

  p {
    margin-top: 8px;
    font-size: 14px;
  }
`;

/**
 * 渲染设置内容
 */
function renderSettingsContent(tab: SettingsTabs): ReactNode {
  switch (tab) {
    // 账号组
    case SettingsTabs.Profile:
      return (
        <>
          <SettingHeader title="个人资料" />
          <ProfileSettings />
        </>
      );

    case SettingsTabs.Stats:
      return (
        <>
          <SettingHeader title="数据统计" />
          <StatsSettings />
        </>
      );

    // 通用组
    case SettingsTabs.Appearance:
      return (
        <>
          <SettingHeader title="外观" />
          <GeneralSettings />
        </>
      );

    case SettingsTabs.ChatAppearance:
      return (
        <>
          <SettingHeader title="聊天外观" />
          <ChatAppearanceSettings />
        </>
      );

    case SettingsTabs.Hotkeys:
      return (
        <>
          <SettingHeader title="快捷键" />
          <HotkeysSettings />
        </>
      );

    // 智能体组
    case SettingsTabs.Providers:
      return (
        <>
          <SettingHeader title="凭证管理" />
          <ProviderPoolPage hideHeader />
        </>
      );

    case SettingsTabs.Assistant:
      return (
        <>
          <SettingHeader title="助理服务" />
          <AssistantSettings />
        </>
      );

    case SettingsTabs.Skills:
      return (
        <>
          <SettingHeader title="技能管理" />
          <ExtensionsSettings />
        </>
      );

    case SettingsTabs.ImageGen:
      return (
        <>
          <SettingHeader title="绘画服务" />
          <ImageGenSettings />
        </>
      );

    case SettingsTabs.Voice:
      return (
        <>
          <SettingHeader title="语音服务" />
          <VoiceSettings />
        </>
      );

    // 系统组
    case SettingsTabs.ApiServer:
      return (
        <>
          <SettingHeader title="团队共享网关（内网）" />
          <ApiServerPage hideHeader />
        </>
      );

    case SettingsTabs.McpServer:
      return (
        <>
          <SettingHeader title="MCP 服务器" />
          <McpPanel hideHeader />
        </>
      );

    case SettingsTabs.Proxy:
      return (
        <>
          <SettingHeader title="网络代理" />
          <ProxySettings />
        </>
      );

    case SettingsTabs.Storage:
      return (
        <>
          <SettingHeader title="数据存储" />
          <div className="space-y-4">
            <DirectorySettings />
            <QuotaSettings />
          </div>
        </>
      );

    case SettingsTabs.Security:
      return (
        <>
          <SettingHeader title="安全设置" />
          <div className="space-y-6">
            <TlsSettings />
            <RemoteManagementSettings />
          </div>
        </>
      );

    case SettingsTabs.ExternalTools:
      return (
        <>
          <SettingHeader title="外部工具" />
          <ExternalToolsSettings />
        </>
      );

    case SettingsTabs.Experimental:
      return (
        <>
          <SettingHeader title="实验功能" />
          <ExperimentalSettings />
        </>
      );

    case SettingsTabs.Developer:
      return (
        <>
          <SettingHeader title="开发者" />
          <DeveloperSettings />
        </>
      );

    case SettingsTabs.About:
      return (
        <>
          <SettingHeader title="关于" />
          <AboutSection />
        </>
      );

    default:
      return (
        <PlaceholderPage>
          <p>页面不存在</p>
        </PlaceholderPage>
      );
  }
}

/**
 * 设置页面主组件
 */
interface SettingsLayoutV2Props {
  onNavigate?: (page: Page, params?: PageParams) => void;
  initialTab?: SettingsTabs;
}

const WIDE_CONTENT_TABS = new Set<SettingsTabs>([
  SettingsTabs.Providers,
  SettingsTabs.ApiServer,
  SettingsTabs.McpServer,
]);

export function SettingsLayoutV2({
  onNavigate,
  initialTab,
}: SettingsLayoutV2Props) {
  const [activeTab, setActiveTab] = useState<SettingsTabs>(
    initialTab || SettingsTabs.Appearance,
  );

  useEffect(() => {
    if (initialTab) {
      setActiveTab(initialTab);
    }
  }, [initialTab]);

  const handleBackToHome = () => {
    if (onNavigate) {
      onNavigate("agent", buildHomeAgentParams());
    }
  };

  return (
    <>
      {/* 顶部返回栏 */}
      <HeaderBar>
        <BackButton onClick={handleBackToHome}>
          <Home />
          返回首页
        </BackButton>
      </HeaderBar>
      {/* 设置内容 */}
      <LayoutContainer>
        <SettingsSidebar activeTab={activeTab} onTabChange={setActiveTab} />
        <ContentContainer>
          <ContentWrapper $wide={WIDE_CONTENT_TABS.has(activeTab)}>
            {renderSettingsContent(activeTab)}
          </ContentWrapper>
        </ContentContainer>
      </LayoutContainer>
    </>
  );
}

export default SettingsLayoutV2;

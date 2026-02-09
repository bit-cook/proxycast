/**
 * 设置页面主布局组件
 *
 * 采用左侧边栏 + 右侧内容的布局
 * 参考 LobeHub 的设置布局设计
 */

import { useState, ReactNode } from "react";
import styled from "styled-components";
import { SettingsSidebar } from "./SettingsSidebar";
import { SettingsTabs } from "@/types/settings";

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
// 连接设置
import { ConnectionsSettings } from "../../settings/ConnectionsSettings";
// 扩展设置
import { ExtensionsSettings } from "../../settings/ExtensionsSettings";
// 快捷键设置
import { HotkeysSettings } from "../general/hotkeys";

import { SettingHeader } from "../features/SettingHeader";

const LayoutContainer = styled.div`
  display: flex;
  height: 100%;
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

const ContentWrapper = styled.div`
  max-width: 800px;
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
          <PlaceholderPage>
            <p>个人资料设置</p>
            <p>即将推出...</p>
          </PlaceholderPage>
        </>
      );

    case SettingsTabs.Stats:
      return (
        <>
          <SettingHeader title="数据统计" />
          <PlaceholderPage>
            <p>使用统计信息</p>
            <p>即将推出...</p>
          </PlaceholderPage>
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
          <PlaceholderPage>
            <p>聊天气泡样式设置</p>
            <p>即将推出...</p>
          </PlaceholderPage>
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
          <SettingHeader title="AI 服务商" />
          <ConnectionsSettings />
        </>
      );

    case SettingsTabs.Assistant:
      return (
        <>
          <SettingHeader title="助理服务" />
          <PlaceholderPage>
            <p>助理配置</p>
            <p>即将推出...</p>
          </PlaceholderPage>
        </>
      );

    case SettingsTabs.Skills:
      return (
        <>
          <SettingHeader title="技能管理" />
          <ExtensionsSettings />
        </>
      );

    case SettingsTabs.Memory:
      return (
        <>
          <SettingHeader title="记忆设置" />
          <PlaceholderPage>
            <p>记忆管理</p>
            <p>即将推出...</p>
          </PlaceholderPage>
        </>
      );

    case SettingsTabs.ImageGen:
      return (
        <>
          <SettingHeader title="绘画服务" />
          <PlaceholderPage>
            <p>绘画服务配置</p>
            <p>即将推出...</p>
          </PlaceholderPage>
        </>
      );

    case SettingsTabs.Voice:
      return (
        <>
          <SettingHeader title="语音服务" />
          <PlaceholderPage>
            <p>语音服务配置</p>
            <p>即将推出...</p>
          </PlaceholderPage>
        </>
      );

    // 系统组
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
export function SettingsLayoutV2() {
  const [activeTab, setActiveTab] = useState<SettingsTabs>(
    SettingsTabs.Appearance,
  );

  return (
    <LayoutContainer>
      <SettingsSidebar activeTab={activeTab} onTabChange={setActiveTab} />
      <ContentContainer>
        <ContentWrapper>{renderSettingsContent(activeTab)}</ContentWrapper>
      </ContentContainer>
    </LayoutContainer>
  );
}

export default SettingsLayoutV2;

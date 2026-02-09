/**
 * 快捷键设置页面
 *
 * 显示和配置应用快捷键
 * 参考 LobeHub 的 Hotkey 设置设计
 */

// import { useState } from 'react';
import styled from "styled-components";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  gap: 24px;
`;

const Section = styled.div`
  display: flex;
  flex-direction: column;
  gap: 12px;
`;

const SectionTitle = styled.h3`
  font-size: 14px;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin: 0;
  padding-bottom: 8px;
  border-bottom: 1px solid hsl(var(--border));
`;

const HotkeyItem = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: 8px;
`;

const HotkeyInfo = styled.div`
  display: flex;
  flex-direction: column;
  gap: 4px;
`;

const HotkeyLabel = styled.div`
  font-size: 14px;
  font-weight: 500;
  color: hsl(var(--foreground));
`;

const HotkeyDescription = styled.div`
  font-size: 12px;
  color: hsl(var(--muted-foreground));
`;

const HotkeyValue = styled.div`
  display: flex;
  gap: 4px;
`;

const KeyBadge = styled.span`
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 8px;
  background: hsl(var(--muted));
  border: 1px solid hsl(var(--border));
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
  color: hsl(var(--foreground));
`;

interface HotkeyConfig {
  id: string;
  label: string;
  description: string;
  keys: string[];
}

const desktopHotkeys: HotkeyConfig[] = [
  {
    id: "toggle-main-window",
    label: "显示/隐藏主窗口",
    description: "全局快捷键显示或隐藏主窗口",
    keys: ["Control", "E"],
  },
  {
    id: "open-settings",
    label: "应用设置",
    description: "打开应用设置页面",
    keys: ["Command Or Control", ","],
  },
];

const essentialHotkeys: HotkeyConfig[] = [
  {
    id: "command-panel",
    label: "命令面板",
    description: "打开全局命令面板快速访问功能",
    keys: ["⌘", "K"],
  },
  {
    id: "search",
    label: "搜索",
    description: "唤起当前页面主要搜索框",
    keys: ["⌘", "J"],
  },
  {
    id: "switch-assistant",
    label: "快捷切换助理",
    description: "通过按住 Ctrl 加数字 0-9 切换固定在侧边栏的助理",
    keys: ["^", "1-9"],
  },
  {
    id: "switch-default-chat",
    label: "切换至默认会话",
    description: "切换至会话标签并进入 Lobe AI",
    keys: ["^", "·"],
  },
  {
    id: "toggle-left-panel",
    label: "显示/隐藏左侧面板",
    description: "显示或隐藏左侧面板",
    keys: ["⌘", "["],
  },
  {
    id: "toggle-right-panel",
    label: "显示/隐藏右侧面板",
    description: "显示或隐藏右侧面板",
    keys: ["⌘", "]"],
  },
];

function HotkeySection({
  title,
  hotkeys,
}: {
  title: string;
  hotkeys: HotkeyConfig[];
}) {
  return (
    <Section>
      <SectionTitle>{title}</SectionTitle>
      {hotkeys.map((hotkey) => (
        <HotkeyItem key={hotkey.id}>
          <HotkeyInfo>
            <HotkeyLabel>{hotkey.label}</HotkeyLabel>
            <HotkeyDescription>{hotkey.description}</HotkeyDescription>
          </HotkeyInfo>
          <HotkeyValue>
            {hotkey.keys.map((key, index) => (
              <KeyBadge key={index}>{key}</KeyBadge>
            ))}
          </HotkeyValue>
        </HotkeyItem>
      ))}
    </Section>
  );
}

export function HotkeysSettings() {
  return (
    <Container>
      <HotkeySection title="桌面端" hotkeys={desktopHotkeys} />
      <HotkeySection title="基础" hotkeys={essentialHotkeys} />
    </Container>
  );
}

export default HotkeysSettings;

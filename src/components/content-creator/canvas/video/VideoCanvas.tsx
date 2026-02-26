import React, { memo, useEffect, useMemo, useState } from "react";
import styled from "styled-components";
import {
  ChevronLeft,
  ChevronRight,
  LayoutGrid,
  PanelLeftClose,
  PanelLeftOpen,
} from "lucide-react";
import { VideoCanvasProps } from "./types";
import { VideoSidebar, type VideoProviderOption } from "./VideoSidebar";
import { VideoWorkspace } from "./VideoWorkspace";
import { apiKeyProviderApi } from "@/lib/api/apiKeyProvider";
import { CanvasBreadcrumbHeader } from "../shared/CanvasBreadcrumbHeader";

const VIDEO_MODEL_PRESETS: Record<string, string[]> = {
  doubao: ["seedance-1-5-pro-251215", "seedance-1-5-lite-250428"],
  volcengine: ["seedance-1-5-pro-251215", "seedance-1-5-lite-250428"],
  dashscope: ["wanx2.1-t2v-turbo", "wanx2.1-kf2v-plus"],
  alibaba: ["wanx2.1-t2v-turbo", "wanx2.1-kf2v-plus"],
  qwen: ["wanx2.1-t2v-turbo", "wanx2.1-kf2v-plus"],
  sora: ["sora-2", "sora-2-pro"],
  openai: ["sora-2", "sora-2-pro"],
  veo: ["veo-3.1"],
  google: ["veo-3.1"],
  vertex: ["veo-3.1"],
  kling: ["kling-2.6"],
  minimax: ["minimax-hailuo-2.3", "minimax-hailuo-02"],
  hailuo: ["minimax-hailuo-2.3", "minimax-hailuo-02"],
  runway: ["runway-gen-4-turbo"],
};

function isVideoProvider(providerId: string): boolean {
  const normalized = providerId.toLowerCase();
  return (
    normalized.includes("doubao") ||
    normalized.includes("volc") ||
    normalized.includes("dashscope") ||
    normalized.includes("alibaba") ||
    normalized.includes("qwen") ||
    normalized.includes("video") ||
    normalized.includes("runway") ||
    normalized.includes("minimax") ||
    normalized.includes("kling") ||
    normalized.includes("sora") ||
    normalized.includes("veo")
  );
}

function resolveProviderModels(provider: VideoProviderOption): string[] {
  if (provider.customModels.length > 0) {
    return provider.customModels;
  }

  const normalizedId = provider.id.toLowerCase();
  for (const [key, models] of Object.entries(VIDEO_MODEL_PRESETS)) {
    if (normalizedId.includes(key)) {
      return models;
    }
  }

  return [];
}

const Root = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  width: 100%;
  padding: 6px 8px 8px;
  gap: 6px;
  background: hsl(var(--muted) / 0.28);
`;

const Body = styled.div`
  display: flex;
  flex: 1;
  min-height: 0;
  width: 100%;
  gap: 6px;
`;

const SidebarContainer = styled.div<{ $collapsed: boolean }>`
  width: ${({ $collapsed }) => ($collapsed ? "0px" : "304px")};
  flex-shrink: 0;
  height: 100%;
  min-height: 0;
  background: hsl(var(--muted) / 0.34);
  border-radius: 12px;
  border: none;
  overflow-y: auto;
  overflow-x: hidden;
  opacity: ${({ $collapsed }) => ($collapsed ? 0 : 1)};
  pointer-events: ${({ $collapsed }) => ($collapsed ? "none" : "auto")};
  transition:
    width 0.2s ease,
    opacity 0.2s ease;
`;

const Splitter = styled.div`
  width: 12px;
  display: flex;
  justify-content: center;
`;

const SplitterButton = styled.button`
  margin-top: 8px;
  width: 16px;
  height: 24px;
  border-radius: 6px;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--muted-foreground));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    color: hsl(var(--foreground));
    border-color: hsl(var(--primary) / 0.4);
  }
`;

const MainContainer = styled.div`
  flex: 1;
  height: 100%;
  min-height: 0;
  background: hsl(var(--background));
  overflow: hidden;
  position: relative;
`;

const WorkspaceFrame = styled.div`
  flex: 1;
  min-height: 0;
  display: flex;
  border-radius: 12px;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  overflow: hidden;
`;

const TopicPanel = styled.div<{ $collapsed: boolean }>`
  position: relative;
  width: ${({ $collapsed }) => ($collapsed ? "0px" : "90px")};
  min-width: ${({ $collapsed }) => ($collapsed ? "0px" : "90px")};
  height: 100%;
  border-left: ${({ $collapsed }) =>
    $collapsed ? "none" : "1px solid hsl(var(--border))"};
  background: hsl(var(--background));
  overflow: visible;
  transition:
    width 0.2s ease,
    min-width 0.2s ease;
`;

const TopicPanelHandle = styled.button`
  position: absolute;
  left: -14px;
  top: 50%;
  transform: translateY(-50%);
  width: 28px;
  height: 50px;
  border: 1px solid hsl(var(--border));
  border-right: none;
  border-radius: 12px 0 0 12px;
  background: hsl(var(--background));
  color: hsl(var(--muted-foreground));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 6;

  &:hover {
    color: hsl(var(--foreground));
  }
`;

const MainAction = styled.button`
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 5;
  width: 20px;
  height: 20px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: default;
`;

export const VideoCanvas: React.FC<VideoCanvasProps> = memo(
  ({ state, onStateChange, projectId, onClose: _onClose, onBackHome }) => {
    const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
    const [topicPanelCollapsed, setTopicPanelCollapsed] = useState(false);
    const [providers, setProviders] = useState<VideoProviderOption[]>([]);

    useEffect(() => {
      let active = true;
      const loadProviders = async () => {
        try {
          const allProviders = await apiKeyProviderApi.getProviders();
          if (!active) {
            return;
          }

          const availableProviders = allProviders
            .filter(
              (provider) =>
                provider.enabled &&
                provider.api_key_count > 0 &&
                isVideoProvider(provider.id),
            )
            .map((provider) => ({
              id: provider.id,
              name: provider.name,
              customModels: provider.custom_models ?? [],
            }));

          setProviders(availableProviders);
        } catch (error) {
          console.error("[VideoCanvas] 加载视频 Provider 失败:", error);
          if (active) {
            setProviders([]);
          }
        }
      };

      void loadProviders();
      return () => {
        active = false;
      };
    }, []);

    const selectedProvider = useMemo(() => {
      return (
        providers.find((provider) => provider.id === state.providerId) ?? null
      );
    }, [providers, state.providerId]);

    const availableModels = useMemo(() => {
      if (!selectedProvider) {
        return [];
      }
      return resolveProviderModels(selectedProvider);
    }, [selectedProvider]);

    useEffect(() => {
      if (providers.length === 0) {
        return;
      }

      if (
        !state.providerId ||
        !providers.some((provider) => provider.id === state.providerId)
      ) {
        const firstProvider = providers[0];
        const firstModel = resolveProviderModels(firstProvider)[0] ?? "";
        onStateChange({
          ...state,
          providerId: firstProvider.id,
          model: firstModel,
        });
        return;
      }

      if (!state.model && availableModels.length > 0) {
        onStateChange({
          ...state,
          model: availableModels[0],
        });
      }
    }, [availableModels, onStateChange, providers, state]);

    return (
      <Root>
        <CanvasBreadcrumbHeader label="视频" onBackHome={onBackHome} />

        <Body>
          <SidebarContainer $collapsed={sidebarCollapsed}>
            <VideoSidebar
              state={state}
              providers={providers}
              availableModels={availableModels}
              onStateChange={onStateChange}
            />
          </SidebarContainer>

          <Splitter>
            <SplitterButton
              onClick={() => setSidebarCollapsed((previous) => !previous)}
              title={sidebarCollapsed ? "展开侧栏" : "收起侧栏"}
            >
              {sidebarCollapsed ? (
                <PanelLeftOpen size={12} />
              ) : (
                <PanelLeftClose size={12} />
              )}
            </SplitterButton>
          </Splitter>

          <WorkspaceFrame>
            <MainContainer>
              <MainAction>
                <LayoutGrid size={12} />
              </MainAction>
              <VideoWorkspace
                state={state}
                projectId={projectId}
                onStateChange={onStateChange}
              />
            </MainContainer>
            <TopicPanel $collapsed={topicPanelCollapsed}>
              <TopicPanelHandle
                type="button"
                title={topicPanelCollapsed ? "展开右侧栏" : "收起右侧栏"}
                onClick={() =>
                  setTopicPanelCollapsed((previous) => !previous)
                }
              >
                {topicPanelCollapsed ? (
                  <ChevronLeft size={12} />
                ) : (
                  <ChevronRight size={12} />
                )}
              </TopicPanelHandle>
            </TopicPanel>
          </WorkspaceFrame>
        </Body>
      </Root>
    );
  },
);

VideoCanvas.displayName = "VideoCanvas";

/**
 * @file 文档画布主组件
 * @description 整合工具栏、渲染器、编辑器、平台标签
 * @module components/content-creator/canvas/document/DocumentCanvas
 */

import React, { memo, useMemo, useCallback, useState, useEffect } from "react";
import styled from "styled-components";
import type { DocumentCanvasProps, ExportFormat, PlatformType } from "./types";
import { DocumentToolbar } from "./DocumentToolbar";
import { DocumentRenderer } from "./DocumentRenderer";
import { NotionEditor } from "./editor";
import { PlatformTabs } from "./PlatformTabs";
import { CanvasBreadcrumbHeader } from "../shared/CanvasBreadcrumbHeader";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  width: 100%;
  padding: 16px;
  gap: 8px;
`;

const InnerContainer = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  background: hsl(var(--background));
  border-radius: 12px;
  border: 1px solid hsl(var(--border));
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
`;

const ContentArea = styled.div`
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
`;

const Toast = styled.div<{ $visible: boolean }>`
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  padding: 12px 24px;
  background: hsl(var(--foreground));
  color: hsl(var(--background));
  border-radius: 8px;
  font-size: 14px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  opacity: ${({ $visible }) => ($visible ? 1 : 0)};
  visibility: ${({ $visible }) => ($visible ? "visible" : "hidden")};
  transition: all 0.3s;
  z-index: 1000;
`;

/**
 * 文档画布主组件
 */
export const DocumentCanvas: React.FC<DocumentCanvasProps> = memo(
  ({
    state,
    onStateChange,
    onBackHome,
    onClose,
    isStreaming = false,
    onSelectionTextChange,
  }) => {
    const [editingContent, setEditingContent] = useState("");
    const [toastMessage, setToastMessage] = useState("");
    const [showToast, setShowToast] = useState(false);

    // 当前版本
    const currentVersion = useMemo(() => {
      return (
        state.versions.find((v) => v.id === state.currentVersionId) || null
      );
    }, [state.versions, state.currentVersionId]);

    useEffect(() => {
      onSelectionTextChange?.("");
    }, [state.currentVersionId, state.isEditing, onSelectionTextChange]);

    // 显示提示
    const showMessage = useCallback((message: string) => {
      setToastMessage(message);
      setShowToast(true);
      setTimeout(() => setShowToast(false), 2000);
    }, []);

    // 切换版本
    const handleVersionChange = useCallback(
      (versionId: string) => {
        const version = state.versions.find((v) => v.id === versionId);
        if (version) {
          onStateChange({
            ...state,
            content: version.content,
            currentVersionId: versionId,
          });
        }
      },
      [state, onStateChange],
    );

    // 进入编辑模式
    const handleEditToggle = useCallback(() => {
      setEditingContent(state.content);
      onStateChange({ ...state, isEditing: true });
    }, [state, onStateChange]);

    // 保存编辑
    const handleSave = useCallback(() => {
      if (editingContent !== state.content) {
        const newVersion = {
          id: crypto.randomUUID(),
          content: editingContent,
          createdAt: Date.now(),
          description: "手动编辑",
        };
        onStateChange({
          ...state,
          content: editingContent,
          versions: [...state.versions, newVersion],
          currentVersionId: newVersion.id,
          isEditing: false,
        });
        showMessage("✅ 保存成功");
      } else {
        onStateChange({ ...state, isEditing: false });
      }
      setEditingContent("");
    }, [editingContent, state, onStateChange, showMessage]);

    // 取消编辑
    const handleCancel = useCallback(() => {
      setEditingContent("");
      onStateChange({ ...state, isEditing: false });
    }, [state, onStateChange]);

    // 导出文档
    const handleExport = useCallback(
      async (format: ExportFormat) => {
        const content = state.content;

        switch (format) {
          case "markdown": {
            const blob = new Blob([content], { type: "text/markdown" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = "document.md";
            a.click();
            URL.revokeObjectURL(url);
            showMessage("📄 已导出 Markdown 文件");
            break;
          }
          case "text": {
            const plainText = content
              .replace(/#{1,6}\s/g, "")
              .replace(/\*\*(.+?)\*\*/g, "$1")
              .replace(/\*(.+?)\*/g, "$1")
              .replace(/`(.+?)`/g, "$1")
              .replace(/\[(.+?)\]\(.+?\)/g, "$1");
            const blob = new Blob([plainText], { type: "text/plain" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = "document.txt";
            a.click();
            URL.revokeObjectURL(url);
            showMessage("📝 已导出纯文本文件");
            break;
          }
          case "clipboard": {
            await navigator.clipboard.writeText(content);
            showMessage("📋 已复制到剪贴板");
            break;
          }
        }
      },
      [state.content, showMessage],
    );

    // 切换平台
    const handlePlatformChange = useCallback(
      (platform: PlatformType) => {
        onStateChange({ ...state, platform });
      },
      [state, onStateChange],
    );

    return (
      <Container>
        <CanvasBreadcrumbHeader label="文档" onBackHome={onBackHome} />

        <InnerContainer>
          <DocumentToolbar
            currentVersion={currentVersion}
            versions={state.versions}
            isEditing={state.isEditing}
            onVersionChange={handleVersionChange}
            onEditToggle={handleEditToggle}
            onSave={handleSave}
            onCancel={handleCancel}
            onExport={handleExport}
            onClose={onClose}
          />

          <ContentArea>
            {state.isEditing ? (
              <NotionEditor
                content={editingContent}
                onChange={setEditingContent}
                onSave={handleSave}
                onCancel={handleCancel}
                onSelectionTextChange={onSelectionTextChange}
              />
            ) : (
              <DocumentRenderer
                content={state.content}
                platform={state.platform}
                isStreaming={isStreaming}
                onSelectionTextChange={onSelectionTextChange}
              />
            )}
          </ContentArea>

          {!state.isEditing && (
            <PlatformTabs
              currentPlatform={state.platform}
              onPlatformChange={handlePlatformChange}
            />
          )}
        </InnerContainer>

        <Toast $visible={showToast}>{toastMessage}</Toast>
      </Container>
    );
  },
);

DocumentCanvas.displayName = "DocumentCanvas";

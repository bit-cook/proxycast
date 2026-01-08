/**
 * 连接配置编辑器模态窗口
 *
 * 提供 Waveterm 风格的 JSON 配置编辑界面。
 * 用于编辑 connections.json 配置文件。
 *
 * @module components/terminal/ConnectionsEditorModal
 */

import { useState, useEffect, useCallback } from "react";
import styled from "styled-components";
import {
  X,
  Save,
  RefreshCw,
  FileJson,
  Check,
  AlertCircle,
  Folder,
} from "lucide-react";
import { createPortal } from "react-dom";
import {
  getRawConnectionConfig,
  saveRawConnectionConfig,
  getConnectionConfigPath,
} from "@/lib/connection-api";

// ============================================================================
// 样式组件
// ============================================================================

const ModalOverlay = styled.div`
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(4px);
`;

const ModalContainer = styled.div`
  width: 90vw;
  max-width: 900px;
  height: 80vh;
  max-height: 700px;
  background: #1a1a1a;
  border: 1px solid #333;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  overflow: hidden;
`;

const ModalHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #333;
  background: #222;
`;

const HeaderTitle = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
  color: #e0e0e0;
  font-size: 16px;
  font-weight: 600;

  svg {
    width: 20px;
    height: 20px;
    color: #58c142;
  }
`;

const HeaderActions = styled.div`
  display: flex;
  align-items: center;
  gap: 8px;
`;

const IconButton = styled.button<{
  $variant?: "default" | "primary" | "danger";
}>`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;

  ${({ $variant }) => {
    switch ($variant) {
      case "primary":
        return `
          background: #58c142;
          color: white;
          &:hover { background: #4aa838; }
          &:disabled { background: #2a2a2a; color: #666; }
        `;
      case "danger":
        return `
          background: transparent;
          color: #808080;
          &:hover { background: rgba(255, 100, 100, 0.1); color: #ff6464; }
        `;
      default:
        return `
          background: transparent;
          color: #808080;
          &:hover { background: rgba(255, 255, 255, 0.1); color: #e0e0e0; }
        `;
    }
  }}

  svg {
    width: 16px;
    height: 16px;
  }
`;

const ModalBody = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
`;

const ConfigPathBar = styled.div`
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background: #1e1e1e;
  border-bottom: 1px solid #2a2a2a;
  color: #888;
  font-size: 12px;
  font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;

  svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
`;

const EditorContainer = styled.div`
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
`;

const TextEditor = styled.textarea`
  flex: 1;
  width: 100%;
  padding: 16px 20px;
  background: #1a1a1a;
  border: none;
  color: #e0e0e0;
  font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;
  font-size: 13px;
  line-height: 1.6;
  resize: none;
  outline: none;
  tab-size: 2;

  &::placeholder {
    color: #555;
  }

  &:focus {
    background: #1c1c1c;
  }
`;

const StatusBar = styled.div<{ $type?: "success" | "error" | "info" }>`
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  border-top: 1px solid #2a2a2a;
  background: #1e1e1e;
  font-size: 12px;

  ${({ $type }) => {
    switch ($type) {
      case "success":
        return `color: #58c142;`;
      case "error":
        return `color: #ff6464;`;
      default:
        return `color: #888;`;
    }
  }}

  svg {
    width: 14px;
    height: 14px;
  }
`;

const ModalFooter = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-top: 1px solid #333;
  background: #222;
`;

const FooterInfo = styled.div`
  font-size: 12px;
  color: #666;
`;

const FooterActions = styled.div`
  display: flex;
  align-items: center;
  gap: 10px;
`;

const Button = styled.button<{ $variant?: "default" | "primary" }>`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;

  ${({ $variant }) =>
    $variant === "primary"
      ? `
          background: #58c142;
          color: white;
          &:hover { background: #4aa838; }
          &:disabled { background: #2a2a2a; color: #666; cursor: not-allowed; }
        `
      : `
          background: #333;
          color: #e0e0e0;
          &:hover { background: #404040; }
        `}

  svg {
    width: 14px;
    height: 14px;
  }
`;

const HelpText = styled.div`
  padding: 16px 20px;
  background: #1e1e1e;
  border-bottom: 1px solid #2a2a2a;
  font-size: 12px;
  color: #888;
  line-height: 1.5;

  code {
    background: #333;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;
    color: #58c142;
  }
`;

// ============================================================================
// 主组件
// ============================================================================

interface ConnectionsEditorModalProps {
  /** 是否打开 */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** 保存成功回调 */
  onSaved?: () => void;
}

export function ConnectionsEditorModal({
  isOpen,
  onClose,
  onSaved,
}: ConnectionsEditorModalProps) {
  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [configPath, setConfigPath] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<{
    type: "success" | "error" | "info";
    message: string;
  } | null>(null);

  // 加载配置
  const loadConfig = useCallback(async () => {
    setLoading(true);
    setStatus(null);
    try {
      const [rawConfig, path] = await Promise.all([
        getRawConnectionConfig(),
        getConnectionConfigPath(),
      ]);
      setContent(rawConfig);
      setOriginalContent(rawConfig);
      setConfigPath(path);
    } catch (err) {
      console.error("[ConnectionsEditorModal] 加载配置失败:", err);
      setStatus({
        type: "error",
        message: `加载配置失败: ${err instanceof Error ? err.message : String(err)}`,
      });
    } finally {
      setLoading(false);
    }
  }, []);

  // 保存配置
  const handleSave = useCallback(async () => {
    setSaving(true);
    setStatus(null);
    try {
      // 验证 JSON 格式
      JSON.parse(content);

      const result = await saveRawConnectionConfig(content);
      if (result.success) {
        setOriginalContent(content);
        setStatus({ type: "success", message: "配置已保存" });
        onSaved?.();
      } else {
        setStatus({
          type: "error",
          message: result.error || "保存失败",
        });
      }
    } catch (err) {
      if (err instanceof SyntaxError) {
        setStatus({
          type: "error",
          message: `JSON 格式错误: ${err.message}`,
        });
      } else {
        setStatus({
          type: "error",
          message: `保存失败: ${err instanceof Error ? err.message : String(err)}`,
        });
      }
    } finally {
      setSaving(false);
    }
  }, [content, onSaved]);

  // 首次打开时加载
  useEffect(() => {
    if (isOpen) {
      loadConfig();
    }
  }, [isOpen, loadConfig]);

  // ESC 键关闭
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
      // Ctrl/Cmd + S 保存
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose, handleSave]);

  // 检查是否有未保存的更改
  const hasChanges = content !== originalContent;

  // 格式化 JSON
  const formatJson = () => {
    try {
      const parsed = JSON.parse(content);
      setContent(JSON.stringify(parsed, null, 2));
      setStatus({ type: "info", message: "JSON 已格式化" });
    } catch {
      setStatus({ type: "error", message: "JSON 格式错误，无法格式化" });
    }
  };

  if (!isOpen) return null;

  return createPortal(
    <ModalOverlay onClick={(e) => e.target === e.currentTarget && onClose()}>
      <ModalContainer onClick={(e) => e.stopPropagation()}>
        <ModalHeader>
          <HeaderTitle>
            <FileJson />
            Wave Config - connections.json
          </HeaderTitle>
          <HeaderActions>
            <IconButton onClick={loadConfig} title="刷新" disabled={loading}>
              <RefreshCw className={loading ? "animate-spin" : ""} />
            </IconButton>
            <IconButton onClick={onClose} $variant="danger" title="关闭">
              <X />
            </IconButton>
          </HeaderActions>
        </ModalHeader>

        <ModalBody>
          <ConfigPathBar>
            <Folder />
            <span>{configPath || "加载中..."}</span>
          </ConfigPathBar>

          <HelpText>
            配置 SSH 连接。格式:{" "}
            <code>{`"connection_name": { "type": "ssh", "user": "root", "host": "192.168.1.1", "port": 22 }`}</code>
          </HelpText>

          <EditorContainer>
            {loading ? (
              <div style={{ padding: 20, color: "#888" }}>加载中...</div>
            ) : (
              <TextEditor
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder='{\n  "connections": {\n    "my-server": {\n      "type": "ssh",\n      "user": "root",\n      "host": "192.168.1.100",\n      "port": 22\n    }\n  }\n}'
                spellCheck={false}
              />
            )}
          </EditorContainer>

          {status && (
            <StatusBar $type={status.type}>
              {status.type === "success" ? (
                <Check />
              ) : status.type === "error" ? (
                <AlertCircle />
              ) : null}
              {status.message}
            </StatusBar>
          )}
        </ModalBody>

        <ModalFooter>
          <FooterInfo>
            {hasChanges ? "有未保存的更改" : "无更改"}
            {" | "}按 Ctrl+S 保存
          </FooterInfo>
          <FooterActions>
            <Button onClick={formatJson}>格式化 JSON</Button>
            <Button onClick={onClose}>取消</Button>
            <Button
              $variant="primary"
              onClick={handleSave}
              disabled={saving || !hasChanges}
            >
              {saving ? (
                <>
                  <RefreshCw className="animate-spin" />
                  保存中...
                </>
              ) : (
                <>
                  <Save />
                  保存
                </>
              )}
            </Button>
          </FooterActions>
        </ModalFooter>
      </ModalContainer>
    </ModalOverlay>,
    document.body,
  );
}

export default ConnectionsEditorModal;

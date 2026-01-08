/**
 * Web 浏览器面板
 *
 * 点击打开按钮后，创建独立的浏览器窗口显示网页。
 * 使用 Tauri WebviewWindow 实现，支持访问任何网站。
 *
 * @module widgets/WebView
 */

import { memo, useState, useCallback, type KeyboardEvent } from "react";
import styled from "styled-components";
import { open } from "@tauri-apps/plugin-shell";
import { Globe, ExternalLink, Info, Rocket, X } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { createWebviewPanel, generatePanelId } from "@/lib/webview-api";

// ============================================================================
// 样式组件 - Waveterm 风格
// ============================================================================

const Container = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  background: rgb(34, 34, 34);
  color: #f7f7f7;
`;

const Toolbar = styled.div`
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.16);
  background: rgb(34, 34, 34);
  flex-shrink: 0;
  min-height: 44px;
  box-sizing: border-box;
`;

const IconButton = styled.button<{ $disabled?: boolean }>`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: ${({ $disabled }) => ($disabled ? "#404040" : "#808080")};
  cursor: ${({ $disabled }) => ($disabled ? "not-allowed" : "pointer")};
  transition: all 0.15s ease;

  &:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #f7f7f7;
  }

  svg {
    width: 16px;
    height: 16px;
  }
`;

const UrlInputWrapper = styled.div`
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 12px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid transparent;
  transition: all 0.15s ease;

  &:focus-within {
    border-color: rgb(88, 193, 66);
    background: rgba(0, 0, 0, 0.7);
  }
`;

const UrlIcon = styled.div`
  display: flex;
  align-items: center;
  color: #808080;

  svg {
    width: 14px;
    height: 14px;
  }
`;

const UrlInput = styled.input`
  flex: 1;
  border: none;
  background: transparent;
  font-size: 13px;
  color: #f7f7f7;
  outline: none;

  &::placeholder {
    color: #666;
  }
`;

const OpenButton = styled.button`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  background: rgb(88, 193, 66);
  color: #000;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.15s ease;

  &:hover {
    opacity: 0.9;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  svg {
    width: 14px;
    height: 14px;
  }
`;

const ContentArea = styled.div`
  flex: 1;
  min-height: 0;
  position: relative;
  display: flex;
  flex-direction: column;
  background: rgba(0, 0, 0, 0.5);
`;

const WelcomePage = styled.div`
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 24px;
  background: rgb(34, 34, 34);
  color: #808080;
  padding: 32px;
`;

const WelcomeIcon = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: rgba(88, 193, 66, 0.1);
  color: rgb(88, 193, 66);

  svg {
    width: 40px;
    height: 40px;
  }
`;

const WelcomeTitle = styled.h2`
  font-size: 20px;
  font-weight: 600;
  color: #f7f7f7;
  margin: 0;
`;

const WelcomeText = styled.p`
  font-size: 14px;
  text-align: center;
  max-width: 400px;
  margin: 0;
  line-height: 1.6;
`;

const InfoBox = styled.div`
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  border-radius: 8px;
  background: rgba(88, 193, 66, 0.1);
  border: 1px solid rgba(88, 193, 66, 0.2);
  max-width: 450px;

  svg {
    width: 20px;
    height: 20px;
    color: rgb(88, 193, 66);
    flex-shrink: 0;
    margin-top: 2px;
  }
`;

const InfoText = styled.span`
  font-size: 13px;
  color: rgb(195, 200, 194);
  line-height: 1.5;
`;

const QuickLinks = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
`;

const QuickLink = styled.button`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.5);
  color: #f7f7f7;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgb(88, 193, 66);
  }

  svg {
    width: 14px;
    height: 14px;
  }
`;

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 确保 URL 有协议前缀
 */
function ensureUrlScheme(url: string): string {
  if (!url) return "";
  url = url.trim();

  // 已有协议
  if (/^(https?|file):\/\//i.test(url)) {
    return url;
  }

  // 本地地址
  if (/^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?/i.test(url)) {
    return `http://${url}`;
  }

  // 看起来像域名
  if (/^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z]{2,})+/i.test(url)) {
    return `https://${url}`;
  }

  // 默认当作搜索
  return `https://www.google.com/search?q=${encodeURIComponent(url)}`;
}

// ============================================================================
// 主组件
// ============================================================================

/**
 * Web 浏览器面板
 *
 * 点击打开按钮后创建独立的浏览器窗口
 */
export const WebView = memo(function WebView() {
  const [inputValue, setInputValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * 在独立窗口中打开 URL
   */
  const openInWebview = useCallback(async (url: string) => {
    const finalUrl = ensureUrlScheme(url);
    if (!finalUrl) return;

    setIsLoading(true);
    setError(null);

    try {
      const panelId = generatePanelId();
      const hostname = new URL(finalUrl).hostname;

      const result = await createWebviewPanel({
        panel_id: panelId,
        url: finalUrl,
        title: hostname,
        x: 0,
        y: 0,
        width: 1024,
        height: 768,
      });

      if (result.success) {
        setInputValue("");
        console.log("[WebView] 独立窗口创建成功:", panelId);
      } else {
        console.error("[WebView] 创建窗口失败:", result.error);
        setError(result.error || "创建失败");
      }
    } catch (err) {
      console.error("[WebView] 打开窗口异常:", err);
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 处理 URL 输入提交
   */
  const handleUrlSubmit = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter" && inputValue.trim()) {
        openInWebview(inputValue);
      }
    },
    [inputValue, openInWebview],
  );

  /**
   * 在系统浏览器中打开
   */
  const openExternal = useCallback(async (url: string) => {
    try {
      await open(ensureUrlScheme(url));
    } catch (e) {
      console.error("[WebView] 打开外部浏览器失败:", e);
    }
  }, []);

  return (
    <Container>
      <Toolbar>
        <UrlInputWrapper>
          <UrlIcon>
            <Globe />
          </UrlIcon>
          <UrlInput
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleUrlSubmit}
            placeholder="输入网址，按 Enter 打开..."
          />
          {inputValue && (
            <IconButton
              onClick={() => setInputValue("")}
              style={{ width: 20, height: 20 }}
            >
              <X size={12} />
            </IconButton>
          )}
        </UrlInputWrapper>

        <OpenButton
          onClick={() => openInWebview(inputValue)}
          disabled={!inputValue.trim() || isLoading}
        >
          <Rocket />
          {isLoading ? "打开中..." : "打开"}
        </OpenButton>

        <Tooltip>
          <TooltipTrigger asChild>
            <IconButton
              onClick={() => {
                const url = inputValue ? ensureUrlScheme(inputValue) : null;
                if (url) openExternal(url);
              }}
            >
              <ExternalLink />
            </IconButton>
          </TooltipTrigger>
          <TooltipContent>在系统浏览器中打开</TooltipContent>
        </Tooltip>
      </Toolbar>

      <ContentArea>
        <WelcomePage>
          <WelcomeIcon>
            <Globe />
          </WelcomeIcon>
          <WelcomeTitle>Web 浏览器</WelcomeTitle>
          <WelcomeText>
            输入网址后点击"打开"，将创建独立的浏览器窗口。 支持访问任何网站，无
            iframe 限制。
          </WelcomeText>

          {error && (
            <InfoBox
              style={{
                background: "rgba(229, 77, 46, 0.1)",
                borderColor: "rgba(229, 77, 46, 0.2)",
              }}
            >
              <Info style={{ color: "rgb(229, 77, 46)" }} />
              <InfoText style={{ color: "rgb(229, 77, 46)" }}>
                错误: {error}
              </InfoText>
            </InfoBox>
          )}

          <InfoBox>
            <Info />
            <InfoText>
              使用 Tauri 原生窗口技术，网页在独立窗口中打开， 可以正常访问
              Google、GitHub 等所有网站。
            </InfoText>
          </InfoBox>

          <QuickLinks>
            <QuickLink onClick={() => openInWebview("https://www.google.com")}>
              <Globe />
              Google
            </QuickLink>
            <QuickLink onClick={() => openInWebview("https://github.com")}>
              <Globe />
              GitHub
            </QuickLink>
            <QuickLink onClick={() => openInWebview("https://chat.openai.com")}>
              <Globe />
              ChatGPT
            </QuickLink>
            <QuickLink onClick={() => openExternal("https://www.google.com")}>
              <ExternalLink />
              系统浏览器
            </QuickLink>
          </QuickLinks>
        </WelcomePage>
      </ContentArea>
    </Container>
  );
});

/**
 * 连接选择器组件
 *
 * 提供 Waveterm 风格的连接下拉选择器。
 * 支持本地连接、SSH 远程连接和 WSL 连接。
 *
 * @module components/terminal/ConnectionSelector
 */

import {
  useState,
  useEffect,
  useCallback,
  useRef,
  type KeyboardEvent,
} from "react";
import styled from "styled-components";
import {
  Monitor,
  Server,
  Terminal,
  Search,
  Settings,
  Loader2,
  Check,
} from "lucide-react";
import {
  listConnections,
  connectionToSessionString,
  type ConnectionListEntry,
  type ConnectionType,
} from "@/lib/connection-api";

// ============================================================================
// 样式组件
// ============================================================================

const SelectorContainer = styled.div`
  position: relative;
  display: inline-flex;
`;

const TriggerButton = styled.button<{ $isOpen?: boolean }>`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #e0e0e0;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  svg {
    width: 14px;
    height: 14px;
    opacity: 0.7;
  }
`;

const TriggerLabel = styled.span`
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
`;

const DropdownPanel = styled.div<{ $visible: boolean }>`
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 320px;
  max-height: 450px;
  background: #222222;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  z-index: 1000;
  overflow: hidden;
  display: ${({ $visible }) => ($visible ? "flex" : "none")};
  flex-direction: column;
`;

const SearchBox = styled.div`
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: #2a2a2a;

  svg {
    width: 16px;
    height: 16px;
    color: #666;
  }
`;

const SearchInput = styled.input`
  flex: 1;
  border: none;
  background: transparent;
  color: #e0e0e0;
  font-size: 14px;
  outline: none;

  &::placeholder {
    color: #666;
  }
`;

const ConnectionList = styled.div`
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
`;

const GroupHeader = styled.div`
  padding: 8px 14px 4px;
  font-size: 11px;
  font-weight: 600;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.5px;
`;

const ConnectionItem = styled.button<{
  $selected?: boolean;
  $focused?: boolean;
}>`
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 14px;
  border: none;
  background: ${({ $selected, $focused }) =>
    $selected
      ? "rgba(88, 193, 66, 0.15)"
      : $focused
        ? "rgba(255, 255, 255, 0.05)"
        : "transparent"};
  color: #e0e0e0;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s ease;

  &:hover {
    background: ${({ $selected }) =>
      $selected ? "rgba(88, 193, 66, 0.2)" : "rgba(255, 255, 255, 0.08)"};
  }
`;

const ConnectionIcon = styled.div<{ $type: string }>`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: ${({ $type }) => ($type === "local" ? "#58c142" : "#888")};

  svg {
    width: 16px;
    height: 16px;
  }
`;

const ConnectionInfo = styled.div`
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
`;

const CheckMark = styled.div<{ $visible: boolean }>`
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: #58c142;
  visibility: ${({ $visible }) => ($visible ? "visible" : "hidden")};

  svg {
    width: 16px;
    height: 16px;
  }
`;

const FooterActions = styled.div`
  display: flex;
  align-items: center;
  padding: 8px 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: #2a2a2a;
`;

const FooterButton = styled.button`
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #888;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #e0e0e0;
  }

  svg {
    width: 14px;
    height: 14px;
  }
`;

const LoadingIndicator = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  color: #666;

  svg {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
`;

const EmptyState = styled.div`
  padding: 24px;
  text-align: center;
  color: #666;
  font-size: 13px;
`;

// ============================================================================
// 辅助函数
// ============================================================================

function getConnectionIcon(type: ConnectionType) {
  switch (type) {
    case "local":
      return <Monitor />;
    case "ssh":
      return <Server />;
    case "wsl":
      return <Terminal />;
    default:
      return <Terminal />;
  }
}

// ============================================================================
// 主组件
// ============================================================================

interface ConnectionSelectorProps {
  /** 当前连接名称 */
  currentConnection?: string;
  /** 选择连接回调 */
  onSelect: (connection: ConnectionListEntry) => void;
  /** 编辑连接回调 - 打开连接编辑器模态窗口 */
  onEditConnections?: () => void;
  /** 打开设置回调（已弃用，改用 onEditConnections） */
  onOpenSettings?: () => void;
}

export function ConnectionSelector({
  currentConnection,
  onSelect,
  onEditConnections,
  onOpenSettings,
}: ConnectionSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [connections, setConnections] = useState<ConnectionListEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [focusedIndex, setFocusedIndex] = useState(-1);

  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // 加载连接列表
  const loadConnections = useCallback(async () => {
    setLoading(true);
    try {
      const list = await listConnections();
      setConnections(list);
    } catch (err) {
      console.error("[ConnectionSelector] 加载连接列表失败:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  // 首次挂载加载连接列表
  useEffect(() => {
    loadConnections();
  }, [loadConnections]);

  // 打开时刷新并聚焦
  useEffect(() => {
    if (isOpen) {
      loadConnections();
      setSearchQuery("");
      setFocusedIndex(-1);
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 50);
    }
  }, [isOpen, loadConnections]);

  // 点击外部关闭
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      return () =>
        document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [isOpen]);

  // 过滤连接
  const filteredConnections = connections.filter((conn) => {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    return (
      conn.name.toLowerCase().includes(query) ||
      conn.label.toLowerCase().includes(query) ||
      (conn.host && conn.host.toLowerCase().includes(query))
    );
  });

  // 分组
  const localConnections = filteredConnections.filter(
    (c) => c.type === "local",
  );
  const remoteConnections = filteredConnections.filter(
    (c) => c.type !== "local",
  );

  // 扁平化列表（用于键盘导航）
  const flatList = [...localConnections, ...remoteConnections];

  // 获取当前选中的连接
  const currentConn =
    connections.find((c) => c.name === currentConnection) ||
    connections.find((c) => c.type === "local");

  // 键盘导航
  const handleKeyDown = (e: KeyboardEvent) => {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setFocusedIndex((prev) => Math.min(prev + 1, flatList.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setFocusedIndex((prev) => Math.max(prev - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        if (focusedIndex >= 0 && focusedIndex < flatList.length) {
          handleSelect(flatList[focusedIndex]);
        }
        break;
      case "Escape":
        e.preventDefault();
        setIsOpen(false);
        break;
    }
  };

  // 选择连接
  const handleSelect = (connection: ConnectionListEntry) => {
    onSelect(connection);
    setIsOpen(false);
  };

  return (
    <SelectorContainer ref={containerRef}>
      <TriggerButton $isOpen={isOpen} onClick={() => setIsOpen(!isOpen)}>
        <Terminal />
        <TriggerLabel>{currentConn?.label || "Local"}</TriggerLabel>
      </TriggerButton>

      <DropdownPanel $visible={isOpen} onKeyDown={handleKeyDown}>
        <SearchBox>
          <Search />
          <SearchInput
            ref={searchInputRef}
            placeholder="Connect to (username@host)..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </SearchBox>

        <ConnectionList>
          {loading ? (
            <LoadingIndicator>
              <Loader2 size={20} />
            </LoadingIndicator>
          ) : filteredConnections.length === 0 ? (
            <EmptyState>
              {searchQuery
                ? "No matching connections"
                : "No connections available"}
            </EmptyState>
          ) : (
            <>
              {localConnections.length > 0 && (
                <>
                  <GroupHeader>Local</GroupHeader>
                  {localConnections.map((conn) => {
                    const isSelected = currentConn?.name === conn.name;
                    return (
                      <ConnectionItem
                        key={conn.name}
                        $selected={isSelected}
                        $focused={flatList.indexOf(conn) === focusedIndex}
                        onClick={() => handleSelect(conn)}
                      >
                        <ConnectionIcon $type={conn.type}>
                          {getConnectionIcon(conn.type)}
                        </ConnectionIcon>
                        <ConnectionInfo>{conn.label}</ConnectionInfo>
                        <CheckMark $visible={isSelected}>
                          <Check />
                        </CheckMark>
                      </ConnectionItem>
                    );
                  })}
                </>
              )}

              {remoteConnections.length > 0 && (
                <>
                  <GroupHeader>Remote</GroupHeader>
                  {remoteConnections.map((conn) => {
                    const isSelected = currentConn?.name === conn.name;
                    return (
                      <ConnectionItem
                        key={conn.name}
                        $selected={isSelected}
                        $focused={flatList.indexOf(conn) === focusedIndex}
                        onClick={() => handleSelect(conn)}
                      >
                        <ConnectionIcon $type={conn.type}>
                          {getConnectionIcon(conn.type)}
                        </ConnectionIcon>
                        <ConnectionInfo>{conn.label}</ConnectionInfo>
                        <CheckMark $visible={isSelected}>
                          <Check />
                        </CheckMark>
                      </ConnectionItem>
                    );
                  })}
                </>
              )}
            </>
          )}
        </ConnectionList>

        {(onEditConnections || onOpenSettings) && (
          <FooterActions>
            <FooterButton
              onClick={() => {
                if (onEditConnections) {
                  onEditConnections();
                } else if (onOpenSettings) {
                  onOpenSettings();
                }
                setIsOpen(false);
              }}
            >
              <Settings />
              Edit Connections
            </FooterButton>
          </FooterActions>
        )}
      </DropdownPanel>
    </SelectorContainer>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export { connectionToSessionString };
export type { ConnectionListEntry };

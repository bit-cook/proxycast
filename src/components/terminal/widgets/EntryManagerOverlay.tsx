/**
 * 文件/文件夹名称编辑覆盖层
 *
 * 借鉴 Waveterm 的 EntryManagerOverlay 组件
 * 用于新建文件、新建文件夹、重命名等操作
 *
 * @module widgets/EntryManagerOverlay
 */

import React, { memo, useState, useCallback, useEffect, useRef } from "react";
import styled from "styled-components";

// ============================================================================
// 类型定义
// ============================================================================

/** 操作类型 */
// eslint-disable-next-line react-refresh/only-export-components
export enum EntryManagerType {
  NewFile = "新建文件",
  NewFolder = "新建文件夹",
  Rename = "重命名",
}

/** 组件属性 */
export interface EntryManagerOverlayProps {
  /** 操作类型 */
  type: EntryManagerType;
  /** 初始值（重命名时使用） */
  initialValue?: string;
  /** 保存回调 */
  onSave: (value: string) => void;
  /** 取消回调 */
  onCancel: () => void;
}

// ============================================================================
// 样式组件
// ============================================================================

const Overlay = styled.div`
  position: fixed;
  inset: 0;
  z-index: 9998;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
`;

const Dialog = styled.div`
  min-width: 320px;
  padding: 20px;
  background: rgb(30, 30, 30);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
`;

const Title = styled.h3`
  margin: 0 0 16px 0;
  font-size: 16px;
  font-weight: 500;
  color: #f0f0f0;
`;

const Input = styled.input`
  width: 100%;
  padding: 10px 12px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.3);
  color: #f0f0f0;
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s ease;

  &:focus {
    border-color: rgb(88, 193, 66);
  }

  &::placeholder {
    color: #606060;
  }
`;

const ButtonGroup = styled.div`
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
`;

const Button = styled.button<{ $primary?: boolean }>`
  padding: 8px 16px;
  border: 1px solid
    ${({ $primary }) =>
      $primary ? "rgb(88, 193, 66)" : "rgba(255, 255, 255, 0.15)"};
  border-radius: 6px;
  background: ${({ $primary }) =>
    $primary ? "rgb(88, 193, 66)" : "transparent"};
  color: ${({ $primary }) => ($primary ? "#000" : "#e0e0e0")};
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;

  &:hover {
    background: ${({ $primary }) =>
      $primary ? "rgb(98, 203, 76)" : "rgba(255, 255, 255, 0.08)"};
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
`;

const ErrorMessage = styled.div`
  margin-top: 8px;
  padding: 8px 12px;
  border-radius: 4px;
  background: rgba(229, 77, 46, 0.15);
  color: #e54d2e;
  font-size: 12px;
`;

// ============================================================================
// 主组件
// ============================================================================

/**
 * 文件/文件夹名称编辑覆盖层
 */
export const EntryManagerOverlay = memo(function EntryManagerOverlay({
  type,
  initialValue = "",
  onSave,
  onCancel,
}: EntryManagerOverlayProps) {
  const [value, setValue] = useState(initialValue);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // 自动聚焦输入框
  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.focus();
      // 重命名时选中文件名（不包括扩展名）
      if (type === EntryManagerType.Rename && initialValue) {
        const dotIndex = initialValue.lastIndexOf(".");
        if (dotIndex > 0) {
          inputRef.current.setSelectionRange(0, dotIndex);
        } else {
          inputRef.current.select();
        }
      }
    }
  }, [type, initialValue]);

  // 验证文件名
  const validateName = useCallback((name: string): string | null => {
    if (!name.trim()) {
      return "名称不能为空";
    }
    if (name.includes("/") || name.includes("\\")) {
      return "名称不能包含 / 或 \\";
    }
    if (name === "." || name === "..") {
      return "名称不能是 . 或 ..";
    }
    return null;
  }, []);

  // 处理保存
  const handleSave = useCallback(() => {
    const trimmedValue = value.trim();
    const validationError = validateName(trimmedValue);
    if (validationError) {
      setError(validationError);
      return;
    }
    onSave(trimmedValue);
  }, [value, validateName, onSave]);

  // 处理键盘事件
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSave();
      } else if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    },
    [handleSave, onCancel],
  );

  // 处理输入变化
  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(e.target.value);
    setError(null);
  }, []);

  // 点击背景关闭
  const handleOverlayClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        onCancel();
      }
    },
    [onCancel],
  );

  // 获取占位符文本
  const placeholder =
    type === EntryManagerType.NewFolder ? "文件夹名称" : "文件名称";

  return (
    <Overlay onClick={handleOverlayClick}>
      <Dialog>
        <Title>{type}</Title>
        <Input
          ref={inputRef}
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
        />
        {error && <ErrorMessage>{error}</ErrorMessage>}
        <ButtonGroup>
          <Button onClick={onCancel}>取消</Button>
          <Button $primary onClick={handleSave} disabled={!value.trim()}>
            确定
          </Button>
        </ButtonGroup>
      </Dialog>
    </Overlay>
  );
});

export default EntryManagerOverlay;

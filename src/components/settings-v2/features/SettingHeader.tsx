/**
 * 设置页头组件
 *
 * 显示设置页面标题和可选的额外操作
 */

import styled from "styled-components";
import { ReactNode } from "react";

const HeaderContainer = styled.div`
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-bottom: 24px;
`;

const TitleRow = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
`;

const Title = styled.h1`
  font-size: 24px;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin: 0;
`;

const Divider = styled.div`
  height: 1px;
  background: hsl(var(--border));
`;

interface SettingHeaderProps {
  /** 页面标题 */
  title: ReactNode;
  /** 额外的操作区域 */
  extra?: ReactNode;
}

/**
 * 设置页头组件
 */
export function SettingHeader({ title, extra }: SettingHeaderProps) {
  return (
    <HeaderContainer>
      <TitleRow>
        <Title>{title}</Title>
        {extra}
      </TitleRow>
      <Divider />
    </HeaderContainer>
  );
}

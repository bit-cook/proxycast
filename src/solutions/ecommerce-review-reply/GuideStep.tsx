/**
 * 电商差评回复 - 配置向导
 *
 * 5 步配置流程:
 * 1. 选择电商平台
 * 2. 配置登录凭证
 * 3. 配置 AI 模型
 * 4. 设置回复规则
 * 5. 测试运行
 */

import { useState } from "react";
import styled from "styled-components";
import type { EcommerceConfig } from "./index";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding: 24px;
  background-color: hsl(var(--card));
  border-radius: 8px;
  border: 1px solid hsl(var(--border));
`;

const StepIndicator = styled.div`
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
`;

const StepDot = styled.div<{ active: boolean; completed: boolean }>`
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 500;
  background-color: ${(props) =>
    props.completed
      ? "hsl(var(--primary))"
      : props.active
        ? "hsl(var(--primary))"
        : "hsl(var(--muted))"};
  color: ${(props) =>
    props.completed || props.active
      ? "hsl(var(--primary-foreground))"
      : "hsl(var(--muted-foreground))"};
`;

const StepTitle = styled.h3`
  font-size: 18px;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin-bottom: 16px;
`;

const FormGroup = styled.div`
  display: flex;
  flex-direction: column;
  gap: 8px;
`;

const Label = styled.label`
  font-size: 14px;
  font-weight: 500;
  color: hsl(var(--foreground));
`;

const Select = styled.select`
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid hsl(var(--border));
  background-color: hsl(var(--background));
  color: hsl(var(--foreground));
  font-size: 14px;

  &:focus {
    outline: none;
    border-color: hsl(var(--primary));
  }
`;

const Input = styled.input`
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid hsl(var(--border));
  background-color: hsl(var(--background));
  color: hsl(var(--foreground));
  font-size: 14px;

  &:focus {
    outline: none;
    border-color: hsl(var(--primary));
  }
`;

const TextArea = styled.textarea`
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid hsl(var(--border));
  background-color: hsl(var(--background));
  color: hsl(var(--foreground));
  font-size: 14px;
  min-height: 100px;
  resize: vertical;

  &:focus {
    outline: none;
    border-color: hsl(var(--primary));
  }
`;

const ButtonGroup = styled.div`
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 16px;
`;

const Button = styled.button<{ variant?: "primary" | "secondary" }>`
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;

  ${(props) =>
    props.variant === "primary"
      ? `
    background-color: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    border: none;

    &:hover {
      opacity: 0.9;
    }
  `
      : `
    background-color: transparent;
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border));

    &:hover {
      background-color: hsl(var(--muted));
    }
  `}

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
`;

const Hint = styled.p`
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  margin-top: 4px;
`;

interface GuideStepProps {
  currentStep: number;
  onStepChange: (step: number) => void;
  onComplete: (config: EcommerceConfig) => void;
}

export function GuideStep({
  currentStep,
  onStepChange,
  onComplete,
}: GuideStepProps) {
  const [platform, setPlatform] = useState<"taobao" | "jd" | "pinduoduo">(
    "taobao",
  );
  const [credType, setCredType] = useState<"cookie" | "password">("cookie");
  const [credValue, setCredValue] = useState("");
  const [primaryModel, setPrimaryModel] = useState("claude-sonnet-4-5");
  const [fallbackModel, setFallbackModel] = useState("");
  const [tone, setTone] = useState<"polite" | "sincere" | "professional">(
    "sincere",
  );
  const [length, setLength] = useState<"short" | "medium" | "long">("medium");
  const [template, setTemplate] = useState("");

  const handleNext = () => {
    if (currentStep < 4) {
      onStepChange(currentStep + 1);
    } else {
      // 完成配置
      const config: EcommerceConfig = {
        platform,
        credentials: {
          type: credType,
          value: credValue,
        },
        aiModel: {
          primary: primaryModel,
          fallback: fallbackModel || undefined,
        },
        replyRules: {
          tone,
          length,
          template: template || undefined,
        },
      };
      onComplete(config);
    }
  };

  const handleBack = () => {
    if (currentStep > 0) {
      onStepChange(currentStep - 1);
    }
  };

  const renderStep = () => {
    switch (currentStep) {
      case 0:
        return (
          <>
            <StepTitle>步骤 1: 选择电商平台</StepTitle>
            <FormGroup>
              <Label>电商平台</Label>
              <Select
                value={platform}
                onChange={(e) => setPlatform(e.target.value as typeof platform)}
              >
                <option value="taobao">淘宝/天猫</option>
                <option value="jd">京东</option>
                <option value="pinduoduo">拼多多</option>
              </Select>
              <Hint>选择您要处理差评的电商平台</Hint>
            </FormGroup>
          </>
        );

      case 1:
        return (
          <>
            <StepTitle>步骤 2: 配置登录凭证</StepTitle>
            <FormGroup>
              <Label>凭证类型</Label>
              <Select
                value={credType}
                onChange={(e) => setCredType(e.target.value as typeof credType)}
              >
                <option value="cookie">Cookie</option>
                <option value="password">账号密码</option>
              </Select>
            </FormGroup>
            <FormGroup>
              <Label>{credType === "cookie" ? "Cookie 值" : "账号密码"}</Label>
              <TextArea
                value={credValue}
                onChange={(e) => setCredValue(e.target.value)}
                placeholder={
                  credType === "cookie" ? "粘贴浏览器 Cookie..." : "账号:密码"
                }
              />
              <Hint>
                {credType === "cookie"
                  ? "从浏览器开发者工具中复制 Cookie"
                  : "输入您的账号和密码,用冒号分隔"}
              </Hint>
            </FormGroup>
          </>
        );

      case 2:
        return (
          <>
            <StepTitle>步骤 3: 配置 AI 模型</StepTitle>
            <FormGroup>
              <Label>主模型</Label>
              <Select
                value={primaryModel}
                onChange={(e) => setPrimaryModel(e.target.value)}
              >
                <option value="claude-sonnet-4-5">Claude Sonnet 4.5</option>
                <option value="claude-opus-4">Claude Opus 4</option>
                <option value="gpt-4">GPT-4</option>
                <option value="gpt-4-turbo">GPT-4 Turbo</option>
              </Select>
              <Hint>用于生成回复的主要 AI 模型</Hint>
            </FormGroup>
            <FormGroup>
              <Label>降级模型 (可选)</Label>
              <Select
                value={fallbackModel}
                onChange={(e) => setFallbackModel(e.target.value)}
              >
                <option value="">不使用降级模型</option>
                <option value="claude-sonnet-3-5">Claude Sonnet 3.5</option>
                <option value="gpt-3.5-turbo">GPT-3.5 Turbo</option>
              </Select>
              <Hint>主模型不可用时使用的备用模型</Hint>
            </FormGroup>
          </>
        );

      case 3:
        return (
          <>
            <StepTitle>步骤 4: 设置回复规则</StepTitle>
            <FormGroup>
              <Label>回复语气</Label>
              <Select
                value={tone}
                onChange={(e) => setTone(e.target.value as typeof tone)}
              >
                <option value="polite">礼貌</option>
                <option value="sincere">真诚</option>
                <option value="professional">专业</option>
              </Select>
            </FormGroup>
            <FormGroup>
              <Label>回复长度</Label>
              <Select
                value={length}
                onChange={(e) => setLength(e.target.value as typeof length)}
              >
                <option value="short">简短 (100-150字)</option>
                <option value="medium">中等 (200-300字)</option>
                <option value="long">详细 (300-500字)</option>
              </Select>
            </FormGroup>
            <FormGroup>
              <Label>自定义模板 (可选)</Label>
              <TextArea
                value={template}
                onChange={(e) => setTemplate(e.target.value)}
                placeholder="输入自定义回复模板..."
              />
              <Hint>留空则使用默认模板</Hint>
            </FormGroup>
          </>
        );

      case 4:
        return (
          <>
            <StepTitle>步骤 5: 测试运行</StepTitle>
            <FormGroup>
              <Label>测试差评链接</Label>
              <Input type="url" placeholder="粘贴差评链接进行测试..." />
              <Hint>输入一个差评链接测试配置是否正常工作</Hint>
            </FormGroup>
            <FormGroup>
              <Label>配置摘要</Label>
              <div
                style={{
                  padding: "12px",
                  backgroundColor: "hsl(var(--muted))",
                  borderRadius: "6px",
                  fontSize: "14px",
                }}
              >
                <p>
                  平台:{" "}
                  {platform === "taobao"
                    ? "淘宝"
                    : platform === "jd"
                      ? "京东"
                      : "拼多多"}
                </p>
                <p>凭证类型: {credType === "cookie" ? "Cookie" : "账号密码"}</p>
                <p>主模型: {primaryModel}</p>
                <p>
                  回复语气:{" "}
                  {tone === "polite"
                    ? "礼貌"
                    : tone === "sincere"
                      ? "真诚"
                      : "专业"}
                </p>
                <p>
                  回复长度:{" "}
                  {length === "short"
                    ? "简短"
                    : length === "medium"
                      ? "中等"
                      : "详细"}
                </p>
              </div>
            </FormGroup>
          </>
        );

      default:
        return null;
    }
  };

  return (
    <Container>
      <StepIndicator>
        {[0, 1, 2, 3, 4].map((step) => (
          <StepDot
            key={step}
            active={step === currentStep}
            completed={step < currentStep}
          >
            {step + 1}
          </StepDot>
        ))}
      </StepIndicator>

      {renderStep()}

      <ButtonGroup>
        {currentStep > 0 && <Button onClick={handleBack}>上一步</Button>}
        <Button variant="primary" onClick={handleNext}>
          {currentStep === 4 ? "完成配置" : "下一步"}
        </Button>
      </ButtonGroup>
    </Container>
  );
}

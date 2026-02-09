/**
 * 电商差评回复解决方案 - 主页面
 *
 * 提供电商差评自动回复功能,支持淘宝、京东、拼多多等平台
 */

import { useState } from "react";
import styled from "styled-components";
import { GuideStep } from "./GuideStep";
import { Tasks } from "./Tasks";
import { Results } from "./Results";
import { ecommerceReviewReplyApi } from "@/lib/api/ecommerce-review-reply";
import type { EcommerceReviewReplyRequest } from "@/lib/api/ecommerce-review-reply";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  background-color: hsl(var(--background));
  padding: 24px;
`;

const Header = styled.div`
  margin-bottom: 24px;
`;

const Title = styled.h1`
  font-size: 24px;
  font-weight: 600;
  color: hsl(var(--foreground));
  margin-bottom: 8px;
`;

const Description = styled.p`
  font-size: 14px;
  color: hsl(var(--muted-foreground));
`;

const Content = styled.div`
  flex: 1;
  display: flex;
  gap: 24px;
  min-height: 0;
`;

const LeftPanel = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
`;

const RightPanel = styled.div`
  width: 400px;
  display: flex;
  flex-direction: column;
  gap: 16px;
`;

export interface EcommerceConfig {
  platform: "taobao" | "jd" | "pinduoduo";
  credentials: {
    type: "cookie" | "password";
    value: string;
  };
  aiModel: {
    primary: string;
    fallback?: string;
  };
  replyRules: {
    tone: "polite" | "sincere" | "professional";
    length: "short" | "medium" | "long";
    template?: string;
  };
}

export interface ReviewTask {
  id: string;
  reviewUrl: string;
  reviewContent?: string;
  status: "pending" | "processing" | "completed" | "failed";
  reply?: string;
  error?: string;
  createdAt: Date;
}

export default function EcommerceReviewReply() {
  const [config, setConfig] = useState<EcommerceConfig | null>(null);
  const [tasks, setTasks] = useState<ReviewTask[]>([]);
  const [currentStep, setCurrentStep] = useState(0);

  const handleConfigComplete = (newConfig: EcommerceConfig) => {
    setConfig(newConfig);
    setCurrentStep(5); // 完成配置,进入任务管理
  };

  const handleAddTask = (reviewUrl: string) => {
    const newTask: ReviewTask = {
      id: Date.now().toString(),
      reviewUrl,
      status: "pending",
      createdAt: new Date(),
    };
    setTasks([...tasks, newTask]);
  };

  const handleExecuteTask = async (taskId: string) => {
    const task = tasks.find((t) => t.id === taskId);
    if (!task || !config) return;

    // 更新任务状态为处理中
    setTasks(
      tasks.map((t) =>
        t.id === taskId ? { ...t, status: "processing" as const } : t,
      ),
    );

    try {
      // 构建请求参数
      const request: EcommerceReviewReplyRequest = {
        platform: config.platform,
        reviewUrl: task.reviewUrl,
        tone: config.replyRules.tone,
        length: config.replyRules.length,
        template: config.replyRules.template,
        model: config.aiModel.primary,
        executionId: taskId,
      };

      // 调用后端 API
      const result = await ecommerceReviewReplyApi.executeReviewReply(request);

      // 更新任务状态
      setTasks(
        tasks.map((t) =>
          t.id === taskId
            ? {
                ...t,
                status: result.success
                  ? ("completed" as const)
                  : ("failed" as const),
                reply: result.output,
                error: result.error,
              }
            : t,
        ),
      );
    } catch (error) {
      // 处理错误
      setTasks(
        tasks.map((t) =>
          t.id === taskId
            ? {
                ...t,
                status: "failed" as const,
                error: error instanceof Error ? error.message : "执行失败",
              }
            : t,
        ),
      );
    }
  };

  return (
    <Container>
      <Header>
        <Title>电商差评回复助手</Title>
        <Description>
          智能分析电商差评并生成专业回复,支持淘宝、京东、拼多多等平台
        </Description>
      </Header>

      <Content>
        <LeftPanel>
          {!config ? (
            <GuideStep
              currentStep={currentStep}
              onStepChange={setCurrentStep}
              onComplete={handleConfigComplete}
            />
          ) : (
            <Tasks
              tasks={tasks}
              config={config}
              onAddTask={handleAddTask}
              onExecuteTask={handleExecuteTask}
            />
          )}
        </LeftPanel>

        <RightPanel>
          <Results tasks={tasks} />
        </RightPanel>
      </Content>
    </Container>
  );
}

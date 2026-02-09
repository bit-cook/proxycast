/**
 * 电商差评回复 - 执行结果
 *
 * 展示任务执行结果和生成的回复
 */

import styled from "styled-components";
import type { ReviewTask } from "./index";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 24px;
  background-color: hsl(var(--card));
  border-radius: 8px;
  border: 1px solid hsl(var(--border));
  height: 100%;
  overflow: hidden;
`;

const Header = styled.div`
  display: flex;
  justify-content: space-between;
  align-items: center;
`;

const Title = styled.h3`
  font-size: 18px;
  font-weight: 600;
  color: hsl(var(--foreground));
`;

const Stats = styled.div`
  display: flex;
  gap: 16px;
  padding: 12px;
  background-color: hsl(var(--muted));
  border-radius: 6px;
`;

const StatItem = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
`;

const StatValue = styled.div`
  font-size: 24px;
  font-weight: 600;
  color: hsl(var(--foreground));
`;

const StatLabel = styled.div`
  font-size: 12px;
  color: hsl(var(--muted-foreground));
`;

const ResultList = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
`;

const ResultItem = styled.div`
  padding: 16px;
  background-color: hsl(var(--background));
  border-radius: 6px;
  border: 1px solid hsl(var(--border));
`;

const ResultHeader = styled.div`
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
`;

const ResultStatus = styled.div<{ status: ReviewTask["status"] }>`
  font-size: 12px;
  font-weight: 500;
  padding: 4px 8px;
  border-radius: 4px;
  background-color: ${(props) => {
    switch (props.status) {
      case "pending":
        return "hsl(var(--muted))";
      case "processing":
        return "hsl(var(--primary) / 0.1)";
      case "completed":
        return "hsl(142, 76%, 36% / 0.1)";
      case "failed":
        return "hsl(var(--destructive) / 0.1)";
    }
  }};
  color: ${(props) => {
    switch (props.status) {
      case "pending":
        return "hsl(var(--muted-foreground))";
      case "processing":
        return "hsl(var(--primary))";
      case "completed":
        return "hsl(142, 76%, 36%)";
      case "failed":
        return "hsl(var(--destructive))";
    }
  }};
`;

const ResultTime = styled.div`
  font-size: 12px;
  color: hsl(var(--muted-foreground));
`;

const ReviewContent = styled.div`
  margin-bottom: 12px;
  padding: 12px;
  background-color: hsl(var(--muted) / 0.5);
  border-radius: 4px;
  font-size: 14px;
  color: hsl(var(--foreground));
`;

const ReplyContent = styled.div`
  padding: 12px;
  background-color: hsl(var(--primary) / 0.05);
  border-radius: 4px;
  border-left: 3px solid hsl(var(--primary));
  font-size: 14px;
  color: hsl(var(--foreground));
  line-height: 1.6;
`;

const ErrorContent = styled.div`
  padding: 12px;
  background-color: hsl(var(--destructive) / 0.05);
  border-radius: 4px;
  border-left: 3px solid hsl(var(--destructive));
  font-size: 14px;
  color: hsl(var(--destructive));
`;

const EmptyState = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  text-align: center;
  color: hsl(var(--muted-foreground));
`;

const CopyButton = styled.button`
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  background-color: transparent;
  color: hsl(var(--primary));
  border: 1px solid hsl(var(--primary));
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    background-color: hsl(var(--primary) / 0.1);
  }
`;

interface ResultsProps {
  tasks: ReviewTask[];
}

export function Results({ tasks }: ResultsProps) {
  const completedTasks = tasks.filter((t) => t.status === "completed");
  const failedTasks = tasks.filter((t) => t.status === "failed");
  const _processingTasks = tasks.filter((t) => t.status === "processing");

  const handleCopyReply = (reply: string) => {
    navigator.clipboard.writeText(reply);
    // TODO: 显示复制成功提示
  };

  const formatTime = (date: Date) => {
    return new Intl.DateTimeFormat("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
    }).format(date);
  };

  const getStatusText = (status: ReviewTask["status"]) => {
    switch (status) {
      case "pending":
        return "待处理";
      case "processing":
        return "处理中";
      case "completed":
        return "已完成";
      case "failed":
        return "失败";
    }
  };

  return (
    <Container>
      <Header>
        <Title>执行结果</Title>
      </Header>

      <Stats>
        <StatItem>
          <StatValue>{tasks.length}</StatValue>
          <StatLabel>总任务</StatLabel>
        </StatItem>
        <StatItem>
          <StatValue>{completedTasks.length}</StatValue>
          <StatLabel>已完成</StatLabel>
        </StatItem>
        <StatItem>
          <StatValue>{failedTasks.length}</StatValue>
          <StatLabel>失败</StatLabel>
        </StatItem>
      </Stats>

      {tasks.length === 0 ? (
        <EmptyState>
          <p>暂无结果</p>
          <p style={{ fontSize: "12px", marginTop: "8px" }}>
            添加任务后查看执行结果
          </p>
        </EmptyState>
      ) : (
        <ResultList>
          {[...tasks]
            .reverse()
            .filter((task) => task.status !== "pending")
            .map((task) => (
              <ResultItem key={task.id}>
                <ResultHeader>
                  <ResultStatus status={task.status}>
                    {getStatusText(task.status)}
                  </ResultStatus>
                  <ResultTime>{formatTime(task.createdAt)}</ResultTime>
                </ResultHeader>

                {task.reviewContent && (
                  <div>
                    <div
                      style={{
                        fontSize: "12px",
                        color: "hsl(var(--muted-foreground))",
                        marginBottom: "4px",
                      }}
                    >
                      差评内容:
                    </div>
                    <ReviewContent>{task.reviewContent}</ReviewContent>
                  </div>
                )}

                {task.reply && (
                  <div>
                    <div
                      style={{
                        fontSize: "12px",
                        color: "hsl(var(--muted-foreground))",
                        marginBottom: "4px",
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <span>生成的回复:</span>
                      <CopyButton onClick={() => handleCopyReply(task.reply!)}>
                        复制
                      </CopyButton>
                    </div>
                    <ReplyContent>{task.reply}</ReplyContent>
                  </div>
                )}

                {task.error && (
                  <div>
                    <div
                      style={{
                        fontSize: "12px",
                        color: "hsl(var(--destructive))",
                        marginBottom: "4px",
                      }}
                    >
                      错误信息:
                    </div>
                    <ErrorContent>{task.error}</ErrorContent>
                  </div>
                )}
              </ResultItem>
            ))}
        </ResultList>
      )}
    </Container>
  );
}

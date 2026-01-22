/**
 * 工作流状态面板组件
 *
 * 显示三阶段工作流的当前状态、进度和统计信息
 */

import React, { useState, useEffect, useCallback } from "react";
import {
  CheckCircle,
  Clock,
  AlertTriangle,
  FileText,
  Settings,
} from "lucide-react";
import {
  ContextMemoryAPI,
  type MemoryStats,
} from "../../../lib/api/contextMemory";
import {
  ToolHooksAPI,
  type HookExecutionStats,
} from "../../../lib/api/toolHooks";

export interface WorkflowStatusPanelProps {
  sessionId: string;
  isWorkflowActive: boolean;
  isWorkflowInitialized: boolean;
  messageCount: number;
  visualOperationCount: number;
  onInitializeWorkflow?: (projectName: string, goal: string) => void;
  onFinalizeWorkflow?: () => void;
  className?: string;
}

interface WorkflowStats {
  memoryStats: MemoryStats | null;
  hookStats: Record<string, HookExecutionStats>;
  isLoading: boolean;
  error: string | null;
}

/**
 * 工作流状态面板组件
 */
export const WorkflowStatusPanel: React.FC<WorkflowStatusPanelProps> = ({
  sessionId,
  isWorkflowActive,
  isWorkflowInitialized,
  messageCount,
  visualOperationCount,
  onInitializeWorkflow,
  onFinalizeWorkflow,
  className = "",
}) => {
  const [stats, setStats] = useState<WorkflowStats>({
    memoryStats: null,
    hookStats: {},
    isLoading: false,
    error: null,
  });

  const [showInitDialog, setShowInitDialog] = useState(false);
  const [projectName, setProjectName] = useState("");
  const [goal, setGoal] = useState("");

  // 加载统计信息
  const loadStats = useCallback(async () => {
    if (!sessionId) return;

    setStats((prev) => ({ ...prev, isLoading: true, error: null }));

    try {
      const [memoryStats, hookStats] = await Promise.all([
        ContextMemoryAPI.getMemoryStats(sessionId),
        ToolHooksAPI.getHookExecutionStats(),
      ]);

      setStats({
        memoryStats,
        hookStats,
        isLoading: false,
        error: null,
      });
    } catch (error) {
      setStats((prev) => ({
        ...prev,
        isLoading: false,
        error: error instanceof Error ? error.message : "加载统计信息失败",
      }));
    }
  }, [sessionId]);

  // 定期刷新统计信息
  useEffect(() => {
    if (isWorkflowInitialized) {
      loadStats();
      const interval = setInterval(loadStats, 10000); // 每10秒刷新
      return () => clearInterval(interval);
    }
  }, [sessionId, isWorkflowInitialized, loadStats]);

  const handleInitializeWorkflow = () => {
    if (projectName.trim() && goal.trim() && onInitializeWorkflow) {
      onInitializeWorkflow(projectName.trim(), goal.trim());
      setShowInitDialog(false);
      setProjectName("");
      setGoal("");
    }
  };

  const getStatusIcon = () => {
    if (!isWorkflowInitialized) {
      return <Clock className="h-5 w-5 text-gray-400" />;
    }
    if (stats.memoryStats && stats.memoryStats.unresolved_errors > 0) {
      return <AlertTriangle className="h-5 w-5 text-yellow-500" />;
    }
    return <CheckCircle className="h-5 w-5 text-green-500" />;
  };

  const getStatusText = () => {
    if (!isWorkflowInitialized) {
      return "工作流未初始化";
    }
    if (!isWorkflowActive) {
      return "工作流已暂停";
    }
    if (stats.memoryStats && stats.memoryStats.unresolved_errors > 0) {
      return `工作流运行中 (${stats.memoryStats.unresolved_errors} 个错误)`;
    }
    return "工作流运行正常";
  };

  return (
    <div
      className={`bg-white border border-gray-200 rounded-lg shadow-sm ${className}`}
    >
      {/* 标题栏 */}
      <div className="px-4 py-3 border-b border-gray-200 flex items-center justify-between">
        <div className="flex items-center space-x-2">
          {getStatusIcon()}
          <h3 className="text-sm font-medium text-gray-900">三阶段工作流</h3>
        </div>
        <div className="flex items-center space-x-2">
          {!isWorkflowInitialized ? (
            <button
              onClick={() => setShowInitDialog(true)}
              className="text-xs px-2 py-1 bg-blue-100 text-blue-700 rounded hover:bg-blue-200"
            >
              初始化
            </button>
          ) : (
            <button
              onClick={onFinalizeWorkflow}
              className="text-xs px-2 py-1 bg-gray-100 text-gray-700 rounded hover:bg-gray-200"
            >
              结束
            </button>
          )}
        </div>
      </div>

      {/* 状态信息 */}
      <div className="p-4 space-y-4">
        {/* 基本状态 */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-600">状态:</span>
          <span
            className={`font-medium ${
              isWorkflowActive ? "text-green-600" : "text-gray-600"
            }`}
          >
            {getStatusText()}
          </span>
        </div>

        {/* 消息统计 */}
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-gray-600">消息数:</span>
            <span className="font-medium">{messageCount}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-gray-600">视觉操作:</span>
            <span className="font-medium">{visualOperationCount}</span>
          </div>
        </div>

        {/* 记忆统计 */}
        {stats.memoryStats && (
          <div className="border-t border-gray-100 pt-3">
            <div className="flex items-center space-x-1 mb-2">
              <FileText className="h-4 w-4 text-gray-400" />
              <span className="text-xs font-medium text-gray-700">
                记忆统计
              </span>
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="flex justify-between">
                <span className="text-gray-600">活跃记忆:</span>
                <span className="font-medium">
                  {stats.memoryStats.active_memories}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">已归档:</span>
                <span className="font-medium">
                  {stats.memoryStats.archived_memories}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">未解决错误:</span>
                <span
                  className={`font-medium ${
                    stats.memoryStats.unresolved_errors > 0
                      ? "text-red-600"
                      : "text-green-600"
                  }`}
                >
                  {stats.memoryStats.unresolved_errors}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">已解决错误:</span>
                <span className="font-medium text-green-600">
                  {stats.memoryStats.resolved_errors}
                </span>
              </div>
            </div>
          </div>
        )}

        {/* 钩子统计 */}
        {Object.keys(stats.hookStats).length > 0 && (
          <div className="border-t border-gray-100 pt-3">
            <div className="flex items-center space-x-1 mb-2">
              <Settings className="h-4 w-4 text-gray-400" />
              <span className="text-xs font-medium text-gray-700">
                钩子统计
              </span>
            </div>
            <div className="space-y-1">
              {Object.entries(stats.hookStats)
                .slice(0, 3)
                .map(([ruleId, stat]) => (
                  <div key={ruleId} className="flex justify-between text-xs">
                    <span className="text-gray-600 truncate">
                      {ruleId.split("-")[0]}:
                    </span>
                    <span className="font-medium">
                      {stat.success_count}/{stat.execution_count}
                    </span>
                  </div>
                ))}
            </div>
          </div>
        )}

        {/* 错误信息 */}
        {stats.error && (
          <div className="text-xs text-red-600 bg-red-50 p-2 rounded">
            {stats.error}
          </div>
        )}

        {/* 加载状态 */}
        {stats.isLoading && (
          <div className="text-xs text-gray-500 text-center">
            加载统计信息...
          </div>
        )}
      </div>

      {/* 初始化对话框 */}
      {showInitDialog && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-96 max-w-full mx-4">
            <h3 className="text-lg font-medium text-gray-900 mb-4">
              初始化工作流
            </h3>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  项目名称
                </label>
                <input
                  type="text"
                  value={projectName}
                  onChange={(e) => setProjectName(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="例如：数据分析项目"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  目标描述
                </label>
                <textarea
                  value={goal}
                  onChange={(e) => setGoal(e.target.value)}
                  rows={3}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="描述你希望完成的目标..."
                />
              </div>
            </div>

            <div className="flex justify-end space-x-3 mt-6">
              <button
                onClick={() => setShowInitDialog(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
              >
                取消
              </button>
              <button
                onClick={handleInitializeWorkflow}
                disabled={!projectName.trim() || !goal.trim()}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                初始化
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default WorkflowStatusPanel;

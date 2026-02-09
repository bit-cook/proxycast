/**
 * 电商差评回复 API
 *
 * 封装电商差评回复相关的 Tauri 命令调用
 */

import { safeInvoke } from "@/lib/dev-bridge";

/**
 * 电商差评回复请求参数
 */
export interface EcommerceReviewReplyRequest {
  /** 电商平台 */
  platform: "taobao" | "jd" | "pinduoduo";
  /** 差评链接 */
  reviewUrl: string;
  /** 回复语气 */
  tone: "polite" | "sincere" | "professional";
  /** 回复长度 */
  length: "short" | "medium" | "long";
  /** 自定义模板 (可选) */
  template?: string;
  /** AI 模型 (可选) */
  model?: string;
  /** 执行 ID (可选) */
  executionId?: string;
}

/**
 * Skill 执行结果
 */
export interface SkillExecutionResult {
  /** 是否成功 */
  success: boolean;
  /** 最终输出 */
  output?: string;
  /** 错误信息 */
  error?: string;
  /** 已完成的步骤结果 */
  stepsCompleted: Array<{
    stepId: string;
    stepName: string;
    success: boolean;
    output?: string;
    error?: string;
  }>;
}

/**
 * 电商差评回复 API
 */
export const ecommerceReviewReplyApi = {
  /**
   * 执行电商差评回复
   *
   * @param request - 请求参数
   * @returns 执行结果
   */
  async executeReviewReply(
    request: EcommerceReviewReplyRequest,
  ): Promise<SkillExecutionResult> {
    return safeInvoke(
      "execute_ecommerce_review_reply",
      request as unknown as Record<string, unknown>,
    );
  },
};

export default ecommerceReviewReplyApi;

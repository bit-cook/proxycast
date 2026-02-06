import type { Dispatch, SetStateAction } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { safeListen } from "@/lib/dev-bridge";
import { parseStreamEvent, type StreamEvent } from "@/lib/api/agent";
import {
  skillExecutionApi,
  type ExecutableSkillInfo,
} from "@/lib/api/skill-execution";
import type { ActionRequired, Message } from "../types";

/** 解析 /skill-name args 命令 */
export interface ParsedSkillCommand {
  skillName: string;
  userInput: string;
}

/** Slash Skill 执行上下文 */
export interface SlashSkillExecutionContext {
  command: ParsedSkillCommand;
  rawContent: string;
  assistantMsgId: string;
  providerType: string;
  model?: string;
  ensureSession: () => Promise<string | null>;
  setMessages: Dispatch<SetStateAction<Message[]>>;
  setIsSending: (value: boolean) => void;
  setCurrentAssistantMsgId: (id: string | null) => void;
  setStreamUnlisten: (unlisten: UnlistenFn | null) => void;
  playTypewriterSound: () => void;
  playToolcallSound: () => void;
  onWriteFile?: (content: string, fileName: string) => void;
}

const VALID_ACTION_TYPES = new Set<ActionRequired["actionType"]>([
  "tool_confirmation",
  "ask_user",
  "elicitation",
]);

/**
 * 解析 slash skill 命令。
 *
 * 格式：`/skill-name` 或 `/skill-name args...`
 */
export function parseSkillSlashCommand(
  content: string,
): ParsedSkillCommand | null {
  const skillMatch = content.match(/^\/([a-zA-Z0-9_-]+)\s*([\s\S]*)$/);
  if (!skillMatch) {
    return null;
  }

  const [, skillName, userInput] = skillMatch;
  return {
    skillName,
    userInput: userInput?.trim() || "",
  };
}

function resolveSkillProviderOverride(
  providerType: string,
  model: string | undefined,
): { providerOverride?: string; modelOverride?: string } {
  const normalizedProvider = providerType.toLowerCase().trim();

  if (!normalizedProvider) {
    return {};
  }

  return {
    providerOverride: providerType,
    modelOverride: model,
  };
}

function normalizeActionType(actionType: string): ActionRequired["actionType"] {
  if (VALID_ACTION_TYPES.has(actionType as ActionRequired["actionType"])) {
    return actionType as ActionRequired["actionType"];
  }
  return "tool_confirmation";
}

function appendTextPart(
  messages: Message[],
  assistantMsgId: string,
  textDelta: string,
) {
  return messages.map((msg) => {
    if (msg.id !== assistantMsgId) return msg;

    const nextParts = [...(msg.contentParts || [])];
    const lastPart = nextParts[nextParts.length - 1];

    if (lastPart && lastPart.type === "text") {
      nextParts[nextParts.length - 1] = {
        type: "text",
        text: lastPart.text + textDelta,
      };
    } else {
      nextParts.push({ type: "text", text: textDelta });
    }

    return {
      ...msg,
      content: (msg.content || "") + textDelta,
      isThinking: false,
      thinkingContent: undefined,
      contentParts: nextParts,
    };
  });
}

function appendThinkingPart(
  messages: Message[],
  assistantMsgId: string,
  textDelta: string,
) {
  return messages.map((msg) => {
    if (msg.id !== assistantMsgId) return msg;

    const nextParts = [...(msg.contentParts || [])];
    const lastPart = nextParts[nextParts.length - 1];

    if (lastPart && lastPart.type === "thinking") {
      nextParts[nextParts.length - 1] = {
        type: "thinking",
        text: lastPart.text + textDelta,
      };
    } else {
      nextParts.push({ type: "thinking", text: textDelta });
    }

    return {
      ...msg,
      isThinking: true,
      thinkingContent: (msg.thinkingContent || "") + textDelta,
      contentParts: nextParts,
    };
  });
}

function tryHandleToolWriteFile(
  toolName: string,
  toolArguments: string | undefined,
  onWriteFile?: (content: string, fileName: string) => void,
) {
  if (!onWriteFile || !toolArguments) {
    return;
  }

  const normalizedToolName = toolName.toLowerCase();
  const looksLikeWriteTool =
    normalizedToolName.includes("write") ||
    normalizedToolName.includes("create");

  if (!looksLikeWriteTool) {
    return;
  }

  try {
    const parsed = JSON.parse(toolArguments) as Record<string, unknown>;
    const filePath =
      (typeof parsed.path === "string" ? parsed.path : undefined) ||
      (typeof parsed.file_path === "string" ? parsed.file_path : undefined) ||
      (typeof parsed.filePath === "string" ? parsed.filePath : undefined);

    const fileContent =
      (typeof parsed.content === "string" ? parsed.content : undefined) ||
      (typeof parsed.text === "string" ? parsed.text : undefined);

    if (filePath && fileContent) {
      onWriteFile(fileContent, filePath);
    }
  } catch (error) {
    console.warn("[SkillCommand] 解析 tool_start 参数失败:", error);
  }
}

async function findMatchedSkill(
  skillName: string,
): Promise<ExecutableSkillInfo | null> {
  try {
    const skills = await skillExecutionApi.listExecutableSkills();
    return skills.find((skill) => skill.name === skillName) || null;
  } catch (error) {
    console.warn("[SkillCommand] 获取可执行 Skills 失败，回退普通对话:", error);
    return null;
  }
}

/**
 * 尝试执行 slash skill 命令。
 *
 * @returns true 表示已处理（包括执行成功或执行失败）；false 表示非 Skill 命令或未命中技能。
 */
export async function tryExecuteSlashSkillCommand(
  ctx: SlashSkillExecutionContext,
): Promise<boolean> {
  const {
    command,
    rawContent,
    assistantMsgId,
    providerType,
    model,
    ensureSession,
    setMessages,
    setIsSending,
    setCurrentAssistantMsgId,
    setStreamUnlisten,
    playTypewriterSound,
    playToolcallSound,
    onWriteFile,
  } = ctx;

  const matchedSkill = await findMatchedSkill(command.skillName);
  if (!matchedSkill) {
    return false;
  }

  const activeSessionId = await ensureSession();
  if (!activeSessionId) {
    setMessages((prev) =>
      prev.map((msg) =>
        msg.id === assistantMsgId
          ? {
              ...msg,
              content: "Skill 执行失败：无法创建会话",
              isThinking: false,
              thinkingContent: undefined,
              contentParts: [
                { type: "text" as const, text: "Skill 执行失败：无法创建会话" },
              ],
            }
          : msg,
      ),
    );
    setIsSending(false);
    setCurrentAssistantMsgId(null);
    return true;
  }

  setMessages((prev) =>
    prev.map((msg) =>
      msg.id === assistantMsgId
        ? {
            ...msg,
            isThinking: true,
            thinkingContent: `正在执行 Skill: ${matchedSkill.display_name}...`,
            content: "",
            contentParts: [],
          }
        : msg,
    ),
  );

  const streamCounters = {
    text_delta: 0,
    thinking_delta: 0,
    tool_start: 0,
    tool_end: 0,
    done: 0,
    final_done: 0,
    error: 0,
  };

  let accumulatedContent = "";
  let skillUnlisten: UnlistenFn | null = null;

  const cleanup = () => {
    if (skillUnlisten) {
      skillUnlisten();
      skillUnlisten = null;
    }
    setStreamUnlisten(null);
    setIsSending(false);
    setCurrentAssistantMsgId(null);
  };

  try {
    const eventName = `skill-exec-${assistantMsgId}`;

    skillUnlisten = await safeListen<StreamEvent>(eventName, ({ payload }) => {
      const streamEvent = parseStreamEvent(payload as unknown);
      if (!streamEvent) return;

      switch (streamEvent.type) {
        case "text_delta": {
          streamCounters.text_delta += 1;
          accumulatedContent += streamEvent.text;
          playTypewriterSound();
          setMessages((prev) =>
            appendTextPart(prev, assistantMsgId, streamEvent.text),
          );
          break;
        }
        case "thinking_delta": {
          streamCounters.thinking_delta += 1;
          setMessages((prev) =>
            appendThinkingPart(prev, assistantMsgId, streamEvent.text),
          );
          break;
        }
        case "tool_start": {
          streamCounters.tool_start += 1;
          playToolcallSound();

          tryHandleToolWriteFile(
            streamEvent.tool_name,
            streamEvent.arguments,
            onWriteFile,
          );

          const newToolCall = {
            id: streamEvent.tool_id,
            name: streamEvent.tool_name,
            arguments: streamEvent.arguments,
            status: "running" as const,
            startTime: new Date(),
          };

          setMessages((prev) =>
            prev.map((msg) => {
              if (msg.id !== assistantMsgId) return msg;
              const existing = msg.toolCalls?.find(
                (tc) => tc.id === streamEvent.tool_id,
              );
              if (existing) return msg;

              return {
                ...msg,
                toolCalls: [...(msg.toolCalls || []), newToolCall],
                contentParts: [
                  ...(msg.contentParts || []),
                  { type: "tool_use" as const, toolCall: newToolCall },
                ],
              };
            }),
          );
          break;
        }
        case "tool_end": {
          streamCounters.tool_end += 1;
          setMessages((prev) =>
            prev.map((msg) => {
              if (msg.id !== assistantMsgId) return msg;

              const updatedToolCalls = (msg.toolCalls || []).map((tc) =>
                tc.id === streamEvent.tool_id
                  ? {
                      ...tc,
                      status: streamEvent.result.success
                        ? ("completed" as const)
                        : ("failed" as const),
                      result: streamEvent.result,
                      endTime: new Date(),
                    }
                  : tc,
              );

              const updatedParts = (msg.contentParts || []).map((part) => {
                if (
                  part.type === "tool_use" &&
                  part.toolCall.id === streamEvent.tool_id
                ) {
                  return {
                    ...part,
                    toolCall: {
                      ...part.toolCall,
                      status: streamEvent.result.success
                        ? ("completed" as const)
                        : ("failed" as const),
                      result: streamEvent.result,
                      endTime: new Date(),
                    },
                  };
                }
                return part;
              });

              return {
                ...msg,
                toolCalls: updatedToolCalls,
                contentParts: updatedParts,
              };
            }),
          );
          break;
        }
        case "action_required": {
          const actionRequired: ActionRequired = {
            requestId: streamEvent.request_id,
            actionType: normalizeActionType(streamEvent.action_type),
            toolName: streamEvent.tool_name,
            arguments: streamEvent.arguments,
            prompt: streamEvent.prompt,
            questions: streamEvent.questions,
            requestedSchema: streamEvent.requested_schema,
          };

          setMessages((prev) =>
            prev.map((msg) => {
              if (msg.id !== assistantMsgId) return msg;
              const existing = msg.actionRequests?.find(
                (item) => item.requestId === streamEvent.request_id,
              );
              if (existing) return msg;

              return {
                ...msg,
                actionRequests: [...(msg.actionRequests || []), actionRequired],
                contentParts: [
                  ...(msg.contentParts || []),
                  { type: "action_required" as const, actionRequired },
                ],
              };
            }),
          );
          break;
        }
        case "done": {
          streamCounters.done += 1;
          break;
        }
        case "final_done": {
          streamCounters.final_done += 1;
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === assistantMsgId
                ? {
                    ...msg,
                    isThinking: false,
                    thinkingContent: undefined,
                    content: accumulatedContent || msg.content,
                  }
                : msg,
            ),
          );
          break;
        }
        case "error": {
          streamCounters.error += 1;
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === assistantMsgId
                ? {
                    ...msg,
                    isThinking: false,
                    thinkingContent: undefined,
                    content:
                      accumulatedContent || `错误: ${streamEvent.message}`,
                  }
                : msg,
            ),
          );
          break;
        }
      }
    });

    setStreamUnlisten(skillUnlisten);

    const { providerOverride, modelOverride } = resolveSkillProviderOverride(
      providerType,
      model,
    );

    const result = await skillExecutionApi.executeSkill({
      skillName: command.skillName,
      userInput: command.userInput || rawContent,
      providerOverride,
      modelOverride,
      executionId: assistantMsgId,
      sessionId: activeSessionId,
    });

    console.log(
      `[SkillCommand] 执行完成: name=${command.skillName}, success=${result.success}, output_len=${result.output?.length ?? 0}, stream_stats=${JSON.stringify(streamCounters)}`,
    );

    const hasStreamedContent = accumulatedContent.trim().length > 0;
    const finalContent = hasStreamedContent
      ? accumulatedContent
      : result.output || result.error || "Skill 执行完成";

    setMessages((prev) =>
      prev.map((msg) => {
        if (msg.id !== assistantMsgId) return msg;

        const nextParts = [...(msg.contentParts || [])];
        if (nextParts.length === 0 && finalContent) {
          nextParts.push({ type: "text", text: finalContent });
        }

        return {
          ...msg,
          content: finalContent,
          isThinking: false,
          thinkingContent: undefined,
          contentParts: nextParts,
        };
      }),
    );

    cleanup();
    return true;
  } catch (error) {
    console.error(`[SkillCommand] 执行失败: ${command.skillName}`, error);

    setMessages((prev) =>
      prev.map((msg) =>
        msg.id === assistantMsgId
          ? {
              ...msg,
              isThinking: false,
              thinkingContent: undefined,
              content: `Skill 执行失败: ${error instanceof Error ? error.message : String(error)}`,
              contentParts: [
                {
                  type: "text",
                  text: `Skill 执行失败: ${error instanceof Error ? error.message : String(error)}`,
                },
              ],
            }
          : msg,
      ),
    );

    cleanup();
    return true;
  }
}

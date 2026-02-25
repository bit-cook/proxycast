/**
 * AI 渠道表单模态框组件
 *
 * 用于添加或编辑 AI 渠道配置
 */

import React, { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Modal, ModalHeader, ModalBody, ModalFooter } from "@/components/Modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type AIChannel,
  type AIChannelConfig,
  type ModelInfo,
  AIProviderEngine,
} from "@/lib/api/channels";

export interface AIChannelFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (config: AIChannelConfig) => Promise<void>;
  initialData?: AIChannel;
}

interface FormState {
  name: string;
  engine: AIProviderEngine;
  display_name: string;
  description: string;
  api_key_env: string;
  base_url: string;
  models_json: string; // JSON 字符串格式存储模型列表
  headers_json: string; // JSON 字符串格式存储请求头
  timeout_seconds: string;
  supports_streaming: boolean;
}

const INITIAL_FORM_STATE: FormState = {
  name: "",
  engine: AIProviderEngine.OPENAI,
  display_name: "",
  description: "",
  api_key_env: "",
  base_url: "",
  models_json: "[]",
  headers_json: "{}",
  timeout_seconds: "60",
  supports_streaming: true,
};

export function AIChannelFormModal({
  isOpen,
  onClose,
  onSubmit,
  initialData,
}: AIChannelFormModalProps) {
  const { t } = useTranslation();
  const [formState, setFormState] = useState<FormState>(INITIAL_FORM_STATE);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // 重置表单或填充初始数据
  useEffect(() => {
    if (isOpen) {
      if (initialData) {
        setFormState({
          name: initialData.name,
          engine: initialData.engine,
          display_name: initialData.display_name,
          description: initialData.description ?? "",
          api_key_env: initialData.api_key_env,
          base_url: initialData.base_url,
          models_json: JSON.stringify(initialData.models, null, 2),
          headers_json: JSON.stringify(initialData.headers ?? {}, null, 2),
          timeout_seconds: String(initialData.timeout_seconds ?? 60),
          supports_streaming: initialData.supports_streaming ?? true,
        });
      } else {
        setFormState(INITIAL_FORM_STATE);
      }
      setErrors({});
    }
  }, [isOpen, initialData]);

  const updateField = useCallback(
    <K extends keyof FormState>(field: K, value: FormState[K]) => {
      setFormState((prev) => ({ ...prev, [field]: value }));
      // 清除该字段的错误
      if (errors[field]) {
        setErrors((prev) => {
          const newErrors = { ...prev };
          delete newErrors[field];
          return newErrors;
        });
      }
    },
    [errors],
  );

  const validateForm = useCallback((): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formState.name.trim()) {
      newErrors.name = t("名称不能为空", "名称不能为空");
    }

    if (!formState.display_name.trim()) {
      newErrors.display_name = t("显示名称不能为空", "显示名称不能为空");
    }

    if (!formState.api_key_env.trim()) {
      newErrors.api_key_env = t("API Key 环境变量名不能为空", "API Key 环境变量名不能为空");
    }

    if (!formState.base_url.trim()) {
      newErrors.base_url = t("API 地址不能为空", "API 地址不能为空");
    } else {
      try {
        new URL(formState.base_url.trim());
      } catch {
        newErrors.base_url = t("请输入有效的 URL", "请输入有效的 URL");
      }
    }

    // 验证 models_json
    try {
      const models = JSON.parse(formState.models_json);
      if (!Array.isArray(models)) {
        newErrors.models_json = t("模型列表必须是数组格式", "模型列表必须是数组格式");
      }
    } catch {
      newErrors.models_json = t("无效的 JSON 格式", "无效的 JSON 格式");
    }

    // 验证 headers_json
    try {
      JSON.parse(formState.headers_json);
    } catch {
      newErrors.headers_json = t("无效的 JSON 格式", "无效的 JSON 格式");
    }

    const timeout = Number(formState.timeout_seconds);
    if (isNaN(timeout) || timeout <= 0) {
      newErrors.timeout_seconds = t("超时时间必须是正整数", "超时时间必须是正整数");
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [formState, t]);

  const handleSubmit = useCallback(async () => {
    if (!validateForm()) {
      return;
    }

    setIsSubmitting(true);
    try {
      const models: ModelInfo[] = JSON.parse(formState.models_json);
      const headers: Record<string, string> = JSON.parse(formState.headers_json);

      const config: AIChannelConfig = {
        name: formState.name.trim(),
        engine: formState.engine,
        display_name: formState.display_name.trim(),
        description: formState.description.trim() || undefined,
        api_key_env: formState.api_key_env.trim(),
        base_url: formState.base_url.trim(),
        models,
        headers: Object.keys(headers).length > 0 ? headers : undefined,
        timeout_seconds: Number(formState.timeout_seconds),
        supports_streaming: formState.supports_streaming,
      };

      await onSubmit(config);
    } catch (e) {
      console.error("提交失败:", e);
      // 错误已经在调用处处理
    } finally {
      setIsSubmitting(false);
    }
  }, [formState, onSubmit, validateForm]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} maxWidth="max-w-2xl">
      <ModalHeader>
        {initialData
          ? t("编辑 AI 渠道", "编辑 AI 渠道")
          : t("添加 AI 渠道", "添加 AI 渠道")}
      </ModalHeader>

      <ModalBody className="space-y-4 max-h-[70vh] overflow-y-auto">
        {/* 名称 */}
        <div className="space-y-1.5">
          <Label htmlFor="name">
            名称 <span className="text-red-500">*</span>
          </Label>
          <Input
            id="name"
            type="text"
            value={formState.name}
            onChange={(e) => updateField("name", e.target.value)}
            placeholder="my-openai-channel"
            disabled={isSubmitting}
            className={errors.name ? "border-red-500" : ""}
          />
          {errors.name && (
            <p className="text-xs text-red-500">{errors.name}</p>
          )}
          <p className="text-xs text-muted-foreground">
            {t("唯一标识符，只能包含小写字母、数字和连字符", "唯一标识符，只能包含小写字母、数字和连字符")}
          </p>
        </div>

        {/* 显示名称 */}
        <div className="space-y-1.5">
          <Label htmlFor="display_name">
            {t("显示名称", "显示名称")} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="display_name"
            type="text"
            value={formState.display_name}
            onChange={(e) => updateField("display_name", e.target.value)}
            placeholder="我的 OpenAI 渠道"
            disabled={isSubmitting}
            className={errors.display_name ? "border-red-500" : ""}
          />
          {errors.display_name && (
            <p className="text-xs text-red-500">{errors.display_name}</p>
          )}
        </div>

        {/* 引擎类型 */}
        <div className="space-y-1.5">
          <Label htmlFor="engine">{t("引擎类型", "引擎类型")}</Label>
          <Select
            value={formState.engine}
            onValueChange={(value) => updateField("engine", value as AIProviderEngine)}
            disabled={isSubmitting}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={AIProviderEngine.OPENAI}>OpenAI</SelectItem>
              <SelectItem value={AIProviderEngine.OLLAMA}>Ollama</SelectItem>
              <SelectItem value={AIProviderEngine.ANTHROPIC}>
                Anthropic
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* 描述 */}
        <div className="space-y-1.5">
          <Label htmlFor="description">{t("描述", "描述")}</Label>
          <Textarea
            id="description"
            value={formState.description}
            onChange={(e) => updateField("description", e.target.value)}
            placeholder={t("可选的渠道描述", "可选的渠道描述")}
            disabled={isSubmitting}
            rows={2}
          />
        </div>

        {/* API Key 环境变量名 */}
        <div className="space-y-1.5">
          <Label htmlFor="api_key_env">
            API Key {t("环境变量名", "环境变量名")} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="api_key_env"
            type="text"
            value={formState.api_key_env}
            onChange={(e) => updateField("api_key_env", e.target.value)}
            placeholder="OPENAI_API_KEY"
            disabled={isSubmitting}
            className={errors.api_key_env ? "border-red-500" : ""}
          />
          {errors.api_key_env && (
            <p className="text-xs text-red-500">{errors.api_key_env}</p>
          )}
          <p className="text-xs text-muted-foreground">
            {t("存储 API Key 的环境变量名称", "存储 API Key 的环境变量名称")}
          </p>
        </div>

        {/* API 地址 */}
        <div className="space-y-1.5">
          <Label htmlFor="base_url">
            API {t("地址", "地址")} <span className="text-red-500">*</span>
          </Label>
          <Input
            id="base_url"
            type="text"
            value={formState.base_url}
            onChange={(e) => updateField("base_url", e.target.value)}
            placeholder="https://api.openai.com/v1"
            disabled={isSubmitting}
            className={errors.base_url ? "border-red-500" : ""}
          />
          {errors.base_url && (
            <p className="text-xs text-red-500">{errors.base_url}</p>
          )}
        </div>

        {/* 模型列表 */}
        <div className="space-y-1.5">
          <Label htmlFor="models_json">{t("模型列表", "模型列表")}</Label>
          <Textarea
            id="models_json"
            value={formState.models_json}
            onChange={(e) => updateField("models_json", e.target.value)}
            placeholder={`[
  {
    "id": "gpt-4",
    "name": "GPT-4",
    "description": "最强大的模型"
  }
]`}
            disabled={isSubmitting}
            rows={6}
            className={errors.models_json ? "border-red-500" : "font-mono text-xs"}
          />
          {errors.models_json && (
            <p className="text-xs text-red-500">{errors.models_json}</p>
          )}
          <p className="text-xs text-muted-foreground">
            {t("JSON 格式的模型列表", "JSON 格式的模型列表")}
          </p>
        </div>

        {/* 自定义请求头 */}
        <div className="space-y-1.5">
          <Label htmlFor="headers_json">{t("自定义请求头", "自定义请求头")}</Label>
          <Textarea
            id="headers_json"
            value={formState.headers_json}
            onChange={(e) => updateField("headers_json", e.target.value)}
            placeholder={`{
  "X-Custom-Header": "value"
}`}
            disabled={isSubmitting}
            rows={3}
            className={errors.headers_json ? "border-red-500" : "font-mono text-xs"}
          />
          {errors.headers_json && (
            <p className="text-xs text-red-500">{errors.headers_json}</p>
          )}
          <p className="text-xs text-muted-foreground">
            {t("JSON 格式的自定义请求头（可选）", "JSON 格式的自定义请求头（可选）")}
          </p>
        </div>

        {/* 超时时间 */}
        <div className="space-y-1.5">
          <Label htmlFor="timeout_seconds">{t("超时时间", "超时时间")}（秒）</Label>
          <Input
            id="timeout_seconds"
            type="number"
            value={formState.timeout_seconds}
            onChange={(e) => updateField("timeout_seconds", e.target.value)}
            min="1"
            disabled={isSubmitting}
            className={errors.timeout_seconds ? "border-red-500" : ""}
          />
          {errors.timeout_seconds && (
            <p className="text-xs text-red-500">{errors.timeout_seconds}</p>
          )}
        </div>

        {/* 支持流式 */}
        <div className="flex items-center justify-between">
          <div>
            <Label>{t("支持流式输出", "支持流式输出")}</Label>
            <p className="text-xs text-muted-foreground">
              {t("该渠道是否支持 SSE 流式响应", "该渠道是否支持 SSE 流式响应")}
            </p>
          </div>
          <Switch
            checked={formState.supports_streaming}
            onCheckedChange={(checked) => updateField("supports_streaming", checked)}
            disabled={isSubmitting}
          />
        </div>
      </ModalBody>

      <ModalFooter>
        <Button variant="outline" onClick={onClose} disabled={isSubmitting}>
          {t("取消", "取消")}
        </Button>
        <Button onClick={handleSubmit} disabled={isSubmitting}>
          {isSubmitting
            ? t("保存中...", "保存中...")
            : initialData
            ? t("保存", "保存")
            : t("添加", "添加")}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

export default AIChannelFormModal;

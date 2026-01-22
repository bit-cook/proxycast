/**
 * 图像分析工具组件
 *
 * 使用 AI 模型分析图片内容，支持视觉理解和描述
 */

import React, { useState, useCallback } from "react";
import {
  Upload,
  Image as ImageIcon,
  Sparkles,
  Loader2,
  X,
  AlertCircle,
} from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { toast } from "sonner";

interface AnalysisResult {
  description?: string;
  analysis?: string;
  error?: string;
}

export function ImageAnalysisTool() {
  const [selectedImage, setSelectedImage] = useState<File | null>(null);
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [analyzing, setAnalyzing] = useState(false);
  const [result, setResult] = useState<AnalysisResult | null>(null);

  const handleImageSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;

      if (!file.type.startsWith("image/")) {
        toast.error("请选择图片文件");
        return;
      }

      if (file.size > 10 * 1024 * 1024) {
        toast.error("图片大小不能超过 10MB");
        return;
      }

      setSelectedImage(file);

      // 创建预览
      const reader = new FileReader();
      reader.onloadend = () => {
        setImagePreview(reader.result as string);
      };
      reader.readAsDataURL(file);

      // 重置结果
      setResult(null);
    },
    [],
  );

  const handleClearImage = useCallback(() => {
    setSelectedImage(null);
    setImagePreview(null);
    setResult(null);
  }, []);

  const handleAnalyze = useCallback(async () => {
    if (!selectedImage) {
      toast.error("请先选择图片");
      return;
    }

    setAnalyzing(true);
    setResult(null);

    try {
      // TODO: 调用后端的 analyze_image API
      // 这里需要实现实际的 API 调用
      // 可能需要通过 Tauri command 或者 MCP 工具调用

      // 模拟 API 调用
      await new Promise((resolve) => setTimeout(resolve, 2000));

      setResult({
        analysis: prompt
          ? `根据您的问题"${prompt}"，这是一张图片的示例分析结果。\n\n实际使用时，这里将显示真实的 AI 分析内容。`
          : "这是一张图片的示例分析结果。\n\n实际使用时，这里将显示真实的 AI 分析内容。",
      });

      toast.success("分析完成");
    } catch (error) {
      console.error("分析失败:", error);
      setResult({
        error: error instanceof Error ? error.message : "分析失败，请重试",
      });
      toast.error("分析失败");
    } finally {
      setAnalyzing(false);
    }
  }, [selectedImage, prompt]);

  const examplePrompts = [
    "描述这张图片的内容",
    "图片中的主要元素是什么？",
    "分析图片的构图和色彩",
    "识别图片中的文字",
  ];

  return (
    <div className="space-y-6 max-w-5xl mx-auto p-6">
      {/* 标题区域 */}
      <div className="text-center space-y-3 pb-6">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/20 to-primary/10 mb-4">
          <ImageIcon className="w-8 h-8 text-primary" />
        </div>
        <h1 className="text-4xl font-bold bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text text-transparent">
          图像分析
        </h1>
        <p className="text-muted-foreground text-lg max-w-2xl mx-auto">
          上传图片，AI 帮您分析内容、识别物体、提取文字
        </p>
        <div className="flex items-center justify-center gap-2">
          <Badge className="bg-primary/10 text-primary border-primary/20">
            AI 驱动
          </Badge>
          <Badge variant="outline" className="border-muted">
            支持多模态
          </Badge>
        </div>
      </div>

      {/* 主内容区域 */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* 左侧：图片上传 */}
        <Card className="shadow-sm border-muted">
          <CardHeader className="pb-4">
            <CardTitle className="text-xl flex items-center gap-2">
              <Upload className="w-5 h-5 text-primary" />
              上传图片
            </CardTitle>
            <CardDescription>
              支持 PNG、JPG、WEBP 等格式，建议小于 2MB
            </CardDescription>
          </CardHeader>
          <CardContent>
            {!imagePreview ? (
              <div className="relative group">
                <input
                  type="file"
                  id="image-upload"
                  accept="image/*"
                  onChange={handleImageSelect}
                  className="hidden"
                />
                <label
                  htmlFor="image-upload"
                  className="flex flex-col items-center justify-center w-full h-80 border-2 border-dashed rounded-xl transition-all duration-200
                         hover:border-primary/50 hover:bg-primary/5 cursor-pointer"
                >
                  <div className="flex flex-col items-center gap-4 text-center p-6">
                    <div className="p-4 rounded-full bg-primary/10 group-hover:bg-primary/20 transition-colors">
                      <Upload className="w-8 h-8 text-primary" />
                    </div>
                    <div className="space-y-1">
                      <p className="font-semibold text-lg">点击或拖拽上传</p>
                      <p className="text-sm text-muted-foreground">
                        支持 PNG、JPG、GIF、WEBP
                      </p>
                    </div>
                    <div className="text-xs text-muted-foreground mt-2">
                      最大文件大小: 10MB
                    </div>
                  </div>
                </label>
              </div>
            ) : (
              <div className="space-y-4">
                <div className="relative group rounded-lg overflow-hidden border bg-muted/30">
                  <img
                    src={imagePreview}
                    alt="Preview"
                    className="w-full h-80 object-contain"
                  />
                  <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity">
                    <Button
                      variant="destructive"
                      size="icon"
                      className="absolute top-3 right-3 shadow-lg"
                      onClick={handleClearImage}
                    >
                      <X className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
                <div className="flex items-center justify-between text-sm px-2">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{selectedImage?.name}</span>
                    <Badge variant="outline" className="text-xs">
                      {selectedImage?.type.split("/")[1]?.toUpperCase()}
                    </Badge>
                  </div>
                  <span className="text-muted-foreground">
                    {((selectedImage?.size || 0) / 1024).toFixed(1)} KB
                  </span>
                </div>
              </div>
            )}
          </CardContent>
        </Card>

        {/* 右侧：分析控制 */}
        {selectedImage && (
          <Card className="shadow-sm border-muted">
            <CardHeader className="pb-4">
              <CardTitle className="text-xl flex items-center gap-2">
                <Sparkles className="w-5 h-5 text-primary" />
                分析设置
              </CardTitle>
              <CardDescription>描述您想了解的图片内容</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* 输入提示 */}
              <div className="space-y-2">
                <label className="text-sm font-medium">分析提示</label>
                <textarea
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="例如：描述这张图片中的主要物体..."
                  className="w-full px-4 py-3 rounded-lg border bg-background resize-none
                           focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none
                           transition-all placeholder:text-muted-foreground/50"
                  rows={4}
                />
              </div>

              {/* 快捷提示 */}
              <div className="space-y-3">
                <label className="text-sm font-medium">快捷提示</label>
                <div className="flex flex-wrap gap-2">
                  {examplePrompts.map((example, index) => (
                    <button
                      key={index}
                      onClick={() => setPrompt(example)}
                      className="px-3 py-2 text-sm rounded-lg border hover:bg-muted hover:border-primary/50
                               transition-all active:scale-95"
                    >
                      {example}
                    </button>
                  ))}
                </div>
              </div>

              {/* 分析按钮 */}
              <Button
                onClick={handleAnalyze}
                disabled={analyzing}
                className="w-full h-12 text-base font-medium shadow-sm hover:shadow transition-all"
                size="lg"
              >
                {analyzing ? (
                  <>
                    <Loader2 className="w-5 h-5 mr-2 animate-spin" />
                    分析中...
                  </>
                ) : (
                  <>
                    <Sparkles className="w-5 h-5 mr-2" />
                    开始分析
                  </>
                )}
              </Button>

              {/* 提示信息 */}
              <div className="flex items-start gap-2 p-3 rounded-lg bg-muted/50 text-sm text-muted-foreground">
                <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                <p>
                  大图片可能导致分析失败，建议使用小于 2MB
                  的图片以获得最佳体验。
                </p>
              </div>
            </CardContent>
          </Card>
        )}
      </div>

      {/* 分析结果 */}
      {result && (
        <Card className="shadow-sm border-muted">
          <CardHeader>
            <CardTitle className="text-xl flex items-center gap-2">
              <Sparkles className="w-5 h-5 text-primary" />
              分析结果
            </CardTitle>
          </CardHeader>
          <CardContent>
            {result.error ? (
              <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-lg">
                <p className="text-destructive whitespace-pre-wrap">
                  {result.error}
                </p>
              </div>
            ) : (
              <div className="prose prose-sm max-w-none dark:prose-invert">
                <div className="p-4 bg-muted/30 rounded-lg border">
                  <p className="whitespace-pre-wrap text-sm leading-relaxed">
                    {result.analysis || result.description}
                  </p>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* 底部说明 */}
      <div className="text-center space-y-2 pt-6">
        <p className="text-sm text-muted-foreground">
          使用 AI 模型进行图片分析，支持物体识别、场景描述、文字提取等功能
        </p>
        <p className="text-xs text-muted-foreground/70">
          分析结果依赖于所选模型的能力和准确性
        </p>
      </div>
    </div>
  );
}

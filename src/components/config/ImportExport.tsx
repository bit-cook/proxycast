import React, { useState, useRef } from "react";
import { Download, Upload, AlertCircle, Check, Shield } from "lucide-react";
import { Config, configApi, ImportResult } from "@/lib/api/config";

interface ImportExportProps {
  config: Config | null;
  onConfigImported: (config: Config) => void;
}

export function ImportExport({ config, onConfigImported }: ImportExportProps) {
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importContent, setImportContent] = useState("");
  const [importResult, setImportResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [redactSecrets, setRedactSecrets] = useState(true);
  const [mergeConfig, setMergeConfig] = useState(true);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Export config (download directly)
  const handleExport = async () => {
    if (!config) return;

    setIsExporting(true);
    setError(null);

    try {
      const result = await configApi.exportConfig(config, redactSecrets);

      // Create download link
      const blob = new Blob([result.content], { type: "text/yaml" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = result.suggested_filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(`导出失败: ${err}`);
    } finally {
      setIsExporting(false);
    }
  };

  // Handle file selection
  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      setImportContent(content);
      setShowImportDialog(true);
      setImportResult(null);
      setError(null);
    };
    reader.onerror = () => {
      setError("读取文件失败");
    };
    reader.readAsText(file);

    // Reset input
    e.target.value = "";
  };

  // Import config
  const handleImport = async () => {
    if (!config || !importContent) return;

    setIsImporting(true);
    setError(null);

    try {
      const result = await configApi.importConfig(
        config,
        importContent,
        mergeConfig,
      );
      setImportResult(result);

      if (result.success) {
        onConfigImported(result.config);
      }
    } catch (err) {
      setError(`导入失败: ${err}`);
    } finally {
      setIsImporting(false);
    }
  };

  // Validate before import
  const handleValidate = async () => {
    if (!importContent) return;

    setError(null);
    try {
      await configApi.validateConfigYaml(importContent);
      setError(null);
    } catch (err) {
      setError(`验证失败: ${err}`);
    }
  };

  return (
    <div className="space-y-6">
      {/* Export Section */}
      <div className="rounded-lg border p-4">
        <h3 className="text-lg font-medium mb-4 flex items-center gap-2">
          <Download className="h-5 w-5" />
          导出配置
        </h3>

        <div className="space-y-4">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={redactSecrets}
              onChange={(e) => setRedactSecrets(e.target.checked)}
              className="rounded border-gray-300"
            />
            <Shield className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm">脱敏敏感信息（API 密钥等）</span>
          </label>

          <div className="flex gap-2">
            <button
              onClick={handleExport}
              disabled={!config || isExporting}
              className="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              <Download className="h-4 w-4" />
              {isExporting ? "导出中..." : "导出配置"}
            </button>
          </div>

          <p className="text-sm text-muted-foreground">
            导出当前配置为 YAML 文件，可用于备份或迁移到其他设备。
          </p>
        </div>
      </div>

      {/* Import Section */}
      <div className="rounded-lg border p-4">
        <h3 className="text-lg font-medium mb-4 flex items-center gap-2">
          <Upload className="h-5 w-5" />
          导入配置
        </h3>

        <div className="space-y-4">
          <input
            ref={fileInputRef}
            type="file"
            accept=".yaml,.yml"
            onChange={handleFileSelect}
            className="hidden"
          />

          <button
            onClick={() => fileInputRef.current?.click()}
            className="flex items-center gap-2 rounded-lg border px-4 py-2 text-sm hover:bg-muted"
          >
            <Upload className="h-4 w-4" />
            选择文件
          </button>

          <p className="text-sm text-muted-foreground">
            从 YAML 文件导入配置，支持合并或替换现有配置。
          </p>
        </div>
      </div>

      {/* Import Dialog */}
      {showImportDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-full max-w-2xl rounded-lg bg-background p-6 shadow-lg">
            <h3 className="text-lg font-medium mb-4">导入配置</h3>

            {/* Preview */}
            <div className="mb-4">
              <label className="text-sm font-medium">配置预览</label>
              <pre className="mt-2 max-h-64 overflow-auto rounded-lg bg-muted p-4 text-sm">
                {importContent.slice(0, 2000)}
                {importContent.length > 2000 && "\n..."}
              </pre>
            </div>

            {/* Options */}
            <div className="mb-4 space-y-2">
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name="importMode"
                  checked={mergeConfig}
                  onChange={() => setMergeConfig(true)}
                  className="rounded-full border-gray-300"
                />
                <span className="text-sm">合并到现有配置</span>
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name="importMode"
                  checked={!mergeConfig}
                  onChange={() => setMergeConfig(false)}
                  className="rounded-full border-gray-300"
                />
                <span className="text-sm">替换现有配置</span>
              </label>
            </div>

            {/* Warnings */}
            {importResult?.warnings && importResult.warnings.length > 0 && (
              <div className="mb-4 rounded-lg border border-yellow-200 bg-yellow-50 p-3 dark:border-yellow-800 dark:bg-yellow-950">
                <p className="text-sm font-medium text-yellow-700 dark:text-yellow-400">
                  警告
                </p>
                <ul className="mt-1 list-disc list-inside text-sm text-yellow-600 dark:text-yellow-500">
                  {importResult.warnings.map((warning, i) => (
                    <li key={i}>{warning}</li>
                  ))}
                </ul>
              </div>
            )}

            {/* Success */}
            {importResult?.success && (
              <div className="mb-4 flex items-center gap-2 rounded-lg border border-green-200 bg-green-50 p-3 text-green-700 dark:border-green-800 dark:bg-green-950 dark:text-green-400">
                <Check className="h-5 w-5" />
                <span className="text-sm">配置导入成功</span>
              </div>
            )}

            {/* Error */}
            {error && (
              <div className="mb-4 flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-3 text-red-700 dark:border-red-800 dark:bg-red-950 dark:text-red-400">
                <AlertCircle className="h-5 w-5 flex-shrink-0" />
                <span className="text-sm">{error}</span>
              </div>
            )}

            {/* Actions */}
            <div className="flex justify-end gap-2">
              <button
                onClick={() => {
                  setShowImportDialog(false);
                  setImportContent("");
                  setImportResult(null);
                  setError(null);
                }}
                className="rounded-lg border px-4 py-2 text-sm hover:bg-muted"
              >
                {importResult?.success ? "关闭" : "取消"}
              </button>
              {!importResult?.success && (
                <>
                  <button
                    onClick={handleValidate}
                    className="rounded-lg border px-4 py-2 text-sm hover:bg-muted"
                  >
                    验证
                  </button>
                  <button
                    onClick={handleImport}
                    disabled={isImporting}
                    className="rounded-lg bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                  >
                    {isImporting ? "导入中..." : "导入"}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Global Error */}
      {error && !showImportDialog && (
        <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-3 text-red-700 dark:border-red-800 dark:bg-red-950 dark:text-red-400">
          <AlertCircle className="h-5 w-5 flex-shrink-0" />
          <span className="text-sm">{error}</span>
        </div>
      )}
    </div>
  );
}

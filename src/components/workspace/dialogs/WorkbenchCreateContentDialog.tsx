import { type CreationMode } from "@/components/content-creator/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import type {
  CreationIntentFieldDefinition,
  CreationIntentFieldKey,
  CreationIntentFormValues,
} from "@/components/workspace/utils/creationIntentPrompt";

export type WorkbenchCreateContentDialogStep = "mode" | "intent";

export interface WorkbenchCreateContentDialogProps {
  open: boolean;
  creatingContent: boolean;
  step: WorkbenchCreateContentDialogStep;
  selectedProjectId: string | null;
  creationModeOptions: Array<{
    value: CreationMode;
    label: string;
    description: string;
  }>;
  selectedCreationMode: CreationMode;
  onCreationModeChange: (mode: CreationMode) => void;
  currentCreationIntentFields: CreationIntentFieldDefinition[];
  creationIntentValues: CreationIntentFormValues;
  onCreationIntentValueChange: (key: CreationIntentFieldKey, value: string) => void;
  currentIntentLength: number;
  minCreationIntentLength: number;
  creationIntentError: string;
  onOpenChange: (open: boolean) => void;
  onBackOrCancel: () => void;
  onGoToIntentStep: () => void;
  onCreateContent: () => void;
}

export function WorkbenchCreateContentDialog({
  open,
  creatingContent,
  step,
  selectedProjectId,
  creationModeOptions,
  selectedCreationMode,
  onCreationModeChange,
  currentCreationIntentFields,
  creationIntentValues,
  onCreationIntentValueChange,
  currentIntentLength,
  minCreationIntentLength,
  creationIntentError,
  onOpenChange,
  onBackOrCancel,
  onGoToIntentStep,
  onCreateContent,
}: WorkbenchCreateContentDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>新建文稿</DialogTitle>
          <DialogDescription>
            {step === "mode"
              ? "先选择创作模式，再填写创作意图。"
              : "填写创作意图后将进入 AI 对话，并按所选模式自动开始写稿。"}
          </DialogDescription>
        </DialogHeader>

        <div className="py-2 space-y-3">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>步骤 {step === "mode" ? "1/2" : "2/2"}</span>
            <span>{step === "mode" ? "选择创作模式" : "填写创作意图"}</span>
          </div>

          {step === "mode" ? (
            <div className="grid gap-2">
              {creationModeOptions.map((modeOption) => (
                <Button
                  key={modeOption.value}
                  type="button"
                  variant={
                    selectedCreationMode === modeOption.value ? "default" : "outline"
                  }
                  className="h-auto justify-start py-3"
                  onClick={() => onCreationModeChange(modeOption.value)}
                  disabled={creatingContent}
                >
                  <div className="text-left">
                    <div className="text-sm font-medium">{modeOption.label}</div>
                    <div
                      className={cn(
                        "text-xs mt-1",
                        selectedCreationMode === modeOption.value
                          ? "text-primary-foreground/80"
                          : "text-muted-foreground",
                      )}
                    >
                      {modeOption.description}
                    </div>
                  </div>
                </Button>
              ))}
            </div>
          ) : (
            <div className="space-y-3">
              {currentCreationIntentFields.map((field) => (
                <div key={field.key} className="grid gap-2">
                  <Label>{field.label}</Label>
                  {field.options && field.options.length > 0 ? (
                    <Select
                      value={creationIntentValues[field.key] || undefined}
                      onValueChange={(value) =>
                        onCreationIntentValueChange(field.key, value)
                      }
                      disabled={creatingContent}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder={field.placeholder} />
                      </SelectTrigger>
                      <SelectContent>
                        {field.options.map((option) => (
                          <SelectItem key={option.value} value={option.value}>
                            {option.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  ) : field.multiline ? (
                    <Textarea
                      id={`creation-intent-${field.key}`}
                      value={creationIntentValues[field.key]}
                      onChange={(event) =>
                        onCreationIntentValueChange(field.key, event.target.value)
                      }
                      placeholder={field.placeholder}
                      className="min-h-[84px] resize-y"
                      disabled={creatingContent}
                    />
                  ) : (
                    <Input
                      id={`creation-intent-${field.key}`}
                      value={creationIntentValues[field.key]}
                      onChange={(event) =>
                        onCreationIntentValueChange(field.key, event.target.value)
                      }
                      placeholder={field.placeholder}
                      disabled={creatingContent}
                    />
                  )}
                </div>
              ))}

              <div className="grid gap-2">
                <Label htmlFor="creation-intent-extra">补充要求</Label>
                <Textarea
                  id="creation-intent-extra"
                  value={creationIntentValues.extraRequirements}
                  onChange={(event) =>
                    onCreationIntentValueChange("extraRequirements", event.target.value)
                  }
                  placeholder="可补充风格、禁忌词、信息来源、输出格式等"
                  className="min-h-[96px] resize-y"
                  disabled={creatingContent}
                />
              </div>

              <div className="space-y-1">
                <p
                  className={cn(
                    "text-xs",
                    currentIntentLength < minCreationIntentLength
                      ? "text-destructive"
                      : "text-muted-foreground",
                  )}
                >
                  创作意图字数：{currentIntentLength}/{minCreationIntentLength}
                </p>
                {creationIntentError && (
                  <p className="text-xs text-destructive">{creationIntentError}</p>
                )}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onBackOrCancel} disabled={creatingContent}>
            {step === "mode" ? "取消" : "上一步"}
          </Button>
          <Button
            onClick={() => {
              if (step === "mode") {
                onGoToIntentStep();
                return;
              }
              onCreateContent();
            }}
            disabled={
              !selectedProjectId ||
              creatingContent ||
              (step === "intent" && currentIntentLength < minCreationIntentLength)
            }
          >
            {step === "mode" ? "下一步" : creatingContent ? "创建中..." : "创建并进入作业"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default WorkbenchCreateContentDialog;

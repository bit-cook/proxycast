/**
 * 删除渠道确认对话框组件
 *
 * 用于确认删除操作
 */

import React from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

export interface DeleteChannelDialogProps {
  isOpen: boolean;
  channelName: string;
  onClose: () => void;
  onConfirm: () => void;
}

export function DeleteChannelDialog({
  isOpen,
  channelName,
  onClose,
  onConfirm,
}: DeleteChannelDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {t("删除确认", "删除确认")}
          </DialogTitle>
          <DialogDescription>
            {t(
              "确定要删除渠道 \"{name}\" 吗？此操作无法撤销。",
              "确定要删除渠道 \"{name}\" 吗？此操作无法撤销。",
              { name: channelName }
            )}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("取消", "取消")}
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            {t("删除", "删除")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default DeleteChannelDialog;

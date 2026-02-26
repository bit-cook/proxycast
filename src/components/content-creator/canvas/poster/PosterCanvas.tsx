/**
 * @file 海报画布主组件
 * @description 基于 Fabric.js 的海报画布编辑器主组件
 * @module components/content-creator/canvas/poster/PosterCanvas
 */

import React, { memo, useRef, useEffect, useState, useCallback } from "react";
import styled from "styled-components";
import { PosterToolbar } from "./PosterToolbar";
import { ElementToolbar } from "./ElementToolbar";
import { LayerPanel } from "./LayerPanel";
import { PageList } from "./PageList";
import { CanvasBreadcrumbHeader } from "../shared/CanvasBreadcrumbHeader";
import {
  useFabricCanvas,
  useElementOperations,
  useHistory,
  usePageOperations,
  useLayerManager,
  useAlignment,
} from "./hooks";
import {
  useTextElement,
  useImageElement,
  useShapeElement,
  useBackgroundElement,
} from "./elements";
import type { PosterCanvasProps, ShapeType } from "./types";
import type { AlignDirection } from "./utils/alignmentGuides";

const Container = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  width: 100%;
  padding: 16px;
  gap: 8px;
`;

const InnerContainer = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  background: hsl(var(--background));
  border-radius: 12px;
  border: 1px solid hsl(var(--border));
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
`;

const MainArea = styled.div`
  display: flex;
  flex: 1;
  overflow: hidden;
  position: relative;
`;

const CanvasWrapper = styled.div`
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: auto;
  padding: 24px;
  position: relative;
`;

const CanvasContainer = styled.div<{ $zoom: number }>`
  position: relative;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  background: #ffffff;
  /* 缩放由 Fabric.js 内部处理，这里不需要 CSS transform */
`;

const GridOverlay = styled.div<{
  $show: boolean;
  $width: number;
  $height: number;
}>`
  position: absolute;
  top: 0;
  left: 0;
  width: ${(props) => props.$width}px;
  height: ${(props) => props.$height}px;
  pointer-events: none;
  opacity: ${(props) => (props.$show ? 0.3 : 0)};
  background-image:
    linear-gradient(to right, hsl(var(--border)) 1px, transparent 1px),
    linear-gradient(to bottom, hsl(var(--border)) 1px, transparent 1px);
  background-size: 20px 20px;
  transition: opacity 0.2s ease;
`;

/**
 * 海报画布主组件
 *
 * 提供图文海报设计、多页编辑和图片导出功能。
 * 基于 Fabric.js 实现可视化编辑。
 *
 * @param props - 组件属性
 * @returns 海报画布组件
 */
export const PosterCanvas: React.FC<PosterCanvasProps> = memo(
  ({ state, onStateChange, onBackHome, onClose }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const wrapperRef = useRef<HTMLDivElement>(null);
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [showLayerPanel, setShowLayerPanel] = useState(false);

    // 获取当前页面
    const currentPage = state.pages[state.currentPageIndex];

    // 使用 Fabric.js 画布 Hook
    const { canvas, zoom, initCanvas, destroyCanvas, setZoom } =
      useFabricCanvas({
        initialZoom: state.zoom,
        onZoomChange: (newZoom) => {
          onStateChange({ ...state, zoom: newZoom });
        },
      });

    // 使用元素操作 Hook
    const {
      selectedIds,
      layers,
      syncLayers,
      selectElement: _selectElement,
    } = useElementOperations({
      canvas,
      onSelectionChange: (ids) => {
        onStateChange({ ...state, selectedLayerIds: ids });
      },
    });

    // 使用图层管理 Hook
    const {
      reorderLayer,
      toggleLayerVisibility,
      toggleLayerLock,
      renameLayer,
      selectLayerElement,
    } = useLayerManager({
      canvas,
      layers,
      onSyncLayers: syncLayers,
      onSelectionChange: (ids) => {
        onStateChange({ ...state, selectedLayerIds: ids });
      },
    });

    // 使用历史记录 Hook
    const { canUndo, canRedo, undo, redo } = useHistory({
      canvas,
      maxHistory: 50,
    });

    // 使用元素 Hooks
    const { addText } = useTextElement({ canvas });
    const { addImageFromFile } = useImageElement({ canvas });
    const { addShape } = useShapeElement({ canvas });
    const { setSolidBackground } = useBackgroundElement({ canvas });

    // 使用页面操作 Hook
    const { addPage, deletePage, duplicatePage, selectPage, reorderPages } =
      usePageOperations({
        pages: state.pages,
        currentPageIndex: state.currentPageIndex,
        onPagesChange: (pages, currentIndex) => {
          onStateChange({ ...state, pages, currentPageIndex: currentIndex });
        },
      });

    // 使用对齐辅助 Hook
    const {
      gridSnapEnabled,
      toggleGridSnap,
      alignSelectedElements,
      enableAlignmentGuides,
      disableAlignmentGuides,
    } = useAlignment({
      canvas,
      canvasWidth: currentPage?.width || 1080,
      canvasHeight: currentPage?.height || 1080,
    });

    /**
     * 初始化画布
     */
    useEffect(() => {
      if (canvasRef.current && !canvas) {
        initCanvas(canvasRef.current, currentPage.width, currentPage.height);
      }

      return () => {
        destroyCanvas();
      };
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    /**
     * 启用对齐辅助
     */
    useEffect(() => {
      if (canvas && state.showGuides) {
        enableAlignmentGuides();
      }
      return () => {
        disableAlignmentGuides();
      };
    }, [
      canvas,
      state.showGuides,
      enableAlignmentGuides,
      disableAlignmentGuides,
    ]);

    /**
     * 当页面尺寸变化时重新初始化画布
     */
    useEffect(() => {
      if (canvas && currentPage) {
        canvas.setWidth(currentPage.width);
        canvas.setHeight(currentPage.height);
        canvas.setBackgroundColor(currentPage.backgroundColor, () => {
          canvas.requestRenderAll();
        });
      }
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [currentPage?.width, currentPage?.height, currentPage?.backgroundColor]);

    /**
     * 同步外部缩放状态到内部
     */
    useEffect(() => {
      if (state.zoom !== zoom) {
        setZoom(state.zoom);
      }
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [state.zoom]);

    /**
     * 处理缩放变更
     */
    const handleZoomChange = useCallback(
      (newZoom: number) => {
        setZoom(newZoom);
      },
      [setZoom],
    );

    /**
     * 切换网格显示
     */
    const handleToggleGrid = useCallback(() => {
      onStateChange({ ...state, showGrid: !state.showGrid });
    }, [state, onStateChange]);

    /**
     * 切换图层面板
     */
    const handleToggleLayerPanel = useCallback(() => {
      setShowLayerPanel((prev) => !prev);
    }, []);

    /**
     * 导出（暂时为空实现）
     */
    const handleExport = useCallback(() => {
      // TODO: 实现导出功能
      console.log("Export clicked");
    }, []);

    /**
     * 处理尺寸变更
     */
    const handleSizeChange = useCallback(
      (width: number, height: number) => {
        const updatedPages = state.pages.map((page, index) => {
          if (index === state.currentPageIndex) {
            return { ...page, width, height };
          }
          return page;
        });
        onStateChange({ ...state, pages: updatedPages });
      },
      [state, onStateChange],
    );

    /**
     * 添加文字
     */
    const handleAddText = useCallback(() => {
      addText();
    }, [addText]);

    /**
     * 添加图片
     */
    const handleAddImage = useCallback(() => {
      fileInputRef.current?.click();
    }, []);

    /**
     * 处理文件选择
     */
    const handleFileChange = useCallback(
      async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
          await addImageFromFile(file);
        }
        e.target.value = "";
      },
      [addImageFromFile],
    );

    /**
     * 添加形状
     */
    const handleAddShape = useCallback(
      (type: ShapeType) => {
        addShape(type);
      },
      [addShape],
    );

    /**
     * 设置背景
     */
    const handleSetBackground = useCallback(() => {
      // 简单实现：切换几个预设颜色
      const colors = ["#ffffff", "#f5f5f5", "#e0e0e0", "#333333"];
      const currentBg = (canvas?.backgroundColor as string) || "#ffffff";
      const currentIndex = colors.indexOf(currentBg);
      const nextIndex = (currentIndex + 1) % colors.length;
      setSolidBackground(colors[nextIndex]);
    }, [canvas, setSolidBackground]);

    /**
     * 处理对齐
     */
    const handleAlign = useCallback(
      (direction: string) => {
        alignSelectedElements(direction as AlignDirection);
      },
      [alignSelectedElements],
    );

    return (
      <Container>
        <CanvasBreadcrumbHeader label="海报" onBackHome={onBackHome} />

        <InnerContainer>
          <PosterToolbar
            zoom={zoom}
            showGrid={state.showGrid}
            canUndo={canUndo}
            canRedo={canRedo}
            canvasWidth={currentPage?.width || 1080}
            canvasHeight={currentPage?.height || 1080}
            onZoomChange={handleZoomChange}
            onToggleGrid={handleToggleGrid}
            onToggleLayerPanel={handleToggleLayerPanel}
            onUndo={undo}
            onRedo={redo}
            onExport={handleExport}
            onSizeChange={handleSizeChange}
            onClose={onClose}
          />

          <MainArea>
            <CanvasWrapper ref={wrapperRef}>
              <CanvasContainer $zoom={zoom}>
                <canvas ref={canvasRef} />
                <GridOverlay
                  $show={state.showGrid}
                  $width={currentPage?.width || 0}
                  $height={currentPage?.height || 0}
                />
              </CanvasContainer>
            </CanvasWrapper>

            {/* 图层面板 */}
            {showLayerPanel && (
              <LayerPanel
                layers={layers}
                selectedIds={selectedIds}
                onSelect={(ids) => {
                  // 选择图层对应的元素
                  if (ids.length === 1) {
                    selectLayerElement(ids[0]);
                  } else if (ids.length > 1) {
                    // 多选时选择第一个
                    selectLayerElement(ids[0]);
                  }
                }}
                onReorder={reorderLayer}
                onToggleVisibility={toggleLayerVisibility}
                onToggleLock={toggleLayerLock}
                onRename={renameLayer}
                onClose={handleToggleLayerPanel}
              />
            )}
          </MainArea>

          <PageList
            pages={state.pages}
            currentIndex={state.currentPageIndex}
            onPageSelect={selectPage}
            onAddPage={addPage}
            onDeletePage={deletePage}
            onDuplicatePage={duplicatePage}
            onReorderPages={reorderPages}
          />

          <ElementToolbar
            onAddText={handleAddText}
            onAddImage={handleAddImage}
            onAddShape={handleAddShape}
            onSetBackground={handleSetBackground}
            hasSelection={selectedIds.length > 0}
            gridSnapEnabled={gridSnapEnabled}
            onAlign={handleAlign}
            onToggleGridSnap={toggleGridSnap}
          />

          {/* 隐藏的文件输入 */}
          <input
            ref={fileInputRef}
            type="file"
            accept="image/jpeg,image/png,image/webp"
            style={{ display: "none" }}
            onChange={handleFileChange}
          />
        </InnerContainer>
      </Container>
    );
  },
);

PosterCanvas.displayName = "PosterCanvas";

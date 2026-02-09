/**
 * 设置页面 V2 导出
 *
 * 新版设置页面，采用 LobeHub 风格的侧边栏布局
 */

export { SettingsLayoutV2 as SettingsPageV2 } from "./_layout";
export { SettingsSidebar } from "./_layout/SettingsSidebar";
export { SettingHeader } from "./features/SettingHeader";
export { useSettingsCategory } from "./hooks/useSettingsCategory";
export type { CategoryItem, CategoryGroup } from "./hooks/useSettingsCategory";

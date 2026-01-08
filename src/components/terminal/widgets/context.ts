/**
 * 小部件上下文定义
 *
 * @module widgets/context
 */

import { createContext } from "react";
import { WidgetContextValue } from "./types";

export const WidgetContext = createContext<WidgetContextValue | null>(null);

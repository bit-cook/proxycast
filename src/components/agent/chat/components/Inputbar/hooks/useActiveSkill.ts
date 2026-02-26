import { useState, useCallback } from "react";
import type { Skill } from "@/lib/api/skills";

export function useActiveSkill() {
  const [activeSkill, setActiveSkill] = useState<Skill | null>(null);

  const wrapTextWithSkill = useCallback(
    (text: string) => {
      if (!activeSkill) return text;
      return `/${activeSkill.key} ${text}`.trim();
    },
    [activeSkill],
  );

  const clearActiveSkill = useCallback(() => setActiveSkill(null), []);

  return { activeSkill, setActiveSkill, wrapTextWithSkill, clearActiveSkill };
}

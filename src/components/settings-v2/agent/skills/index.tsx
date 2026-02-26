import { SkillsPage } from "@/components/skills/SkillsPage";

export function ExtensionsSettings() {
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold">技能管理</h3>
        <p className="text-muted-foreground text-sm">
          管理 Skills 实验功能，不影响核心使用，
          <a
            href="https://github.com/aiclientproxy/proxycast/issues"
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary hover:underline"
          >
            问题反馈
          </a>
        </p>
      </div>

      <div className="pt-1">
        <SkillsPage hideHeader />
      </div>
    </div>
  );
}

import type { ReactNode } from "react";

export interface WorkspaceShellProps {
  header: ReactNode;
  leftSidebar?: ReactNode;
  main: ReactNode;
  rightRail?: ReactNode;
}

export function WorkspaceShell({
  header,
  leftSidebar,
  main,
  rightRail,
}: WorkspaceShellProps) {
  return (
    <div className="flex flex-col h-full min-h-0">
      {header}
      <div className="flex flex-1 min-h-0">
        {leftSidebar}
        <main className="flex-1 min-w-0 min-h-0 flex flex-col">{main}</main>
        {rightRail}
      </div>
    </div>
  );
}

export default WorkspaceShell;

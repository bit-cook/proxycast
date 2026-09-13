use std::borrow::Cow;

use crate::slash_command::SlashCommand;
use std::env;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Locale {
    ZhCn,
    ZhTw,
    #[default]
    EnUs,
    JaJp,
    KoKr,
}

impl Locale {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().replace('_', "-").to_ascii_lowercase();
        if normalized.is_empty() {
            return None;
        }
        if normalized == "zh-tw"
            || normalized.starts_with("zh-hant")
            || normalized.starts_with("zh-tw-")
        {
            return Some(Self::ZhTw);
        }
        if normalized == "zh"
            || normalized == "zh-cn"
            || normalized.starts_with("zh-hans")
            || normalized.starts_with("zh-cn-")
        {
            return Some(Self::ZhCn);
        }
        if normalized == "ja" || normalized.starts_with("ja-") {
            return Some(Self::JaJp);
        }
        if normalized == "ko" || normalized.starts_with("ko-") {
            return Some(Self::KoKr);
        }
        if normalized == "en" || normalized.starts_with("en-") {
            return Some(Self::EnUs);
        }
        None
    }

    pub(crate) fn resolve(explicit: Option<&str>) -> Self {
        explicit
            .and_then(Self::parse)
            .or_else(|| {
                env::var("LIME_LOCALE")
                    .ok()
                    .and_then(|value| Self::parse(&value))
            })
            .or_else(|| {
                env::var("LC_ALL")
                    .ok()
                    .and_then(|value| Self::parse(&value))
            })
            .or_else(|| env::var("LANG").ok().and_then(|value| Self::parse(&value)))
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::EnUs => "en-US",
            Self::JaJp => "ja-JP",
            Self::KoKr => "ko-KR",
        }
    }

    pub(crate) fn model_label(self) -> &'static str {
        match self {
            Self::ZhCn | Self::ZhTw => "模型",
            Self::EnUs => "model",
            Self::JaJp => "モデル",
            Self::KoKr => "모델",
        }
    }

    pub(crate) fn effort_label(self) -> &'static str {
        match self {
            Self::ZhCn | Self::ZhTw => "推理",
            Self::EnUs => "effort",
            Self::JaJp => "推論",
            Self::KoKr => "추론",
        }
    }

    pub(crate) fn permissions_label(self) -> &'static str {
        match self {
            Self::ZhCn => "权限",
            Self::ZhTw => "權限",
            Self::EnUs => "permissions",
            Self::JaJp => "権限",
            Self::KoKr => "권한",
        }
    }

    pub(crate) fn ready_label(self) -> &'static str {
        match self {
            Self::ZhCn => "就绪",
            Self::ZhTw => "就緒",
            Self::EnUs => "ready",
            Self::JaJp => "準備完了",
            Self::KoKr => "준비됨",
        }
    }

    pub(crate) fn working_label(self) -> &'static str {
        match self {
            Self::ZhCn => "处理中",
            Self::ZhTw => "處理中",
            Self::EnUs => "Working",
            Self::JaJp => "処理中",
            Self::KoKr => "작업 중",
        }
    }

    pub(crate) fn interrupt_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "Esc 中断",
            Self::ZhTw => "Esc 中斷",
            Self::EnUs => "esc to interrupt",
            Self::JaJp => "Esc で中断",
            Self::KoKr => "Esc로 중단",
        }
    }

    pub(crate) fn turn_label(self) -> &'static str {
        match self {
            Self::ZhCn | Self::ZhTw => "回合",
            Self::EnUs => "turn",
            Self::JaJp => "ターン",
            Self::KoKr => "턴",
        }
    }

    pub(crate) fn history_search_label(self) -> &'static str {
        match self {
            Self::ZhCn => "反向搜索：",
            Self::ZhTw => "反向搜尋：",
            Self::EnUs => "reverse-i-search: ",
            Self::JaJp => "履歴逆検索: ",
            Self::KoKr => "기록 역검색: ",
        }
    }

    pub(crate) fn file_search_loading(self) -> &'static str {
        match self {
            Self::ZhCn => "正在搜索...",
            Self::ZhTw => "正在搜尋...",
            Self::EnUs => "loading...",
            Self::JaJp => "検索中...",
            Self::KoKr => "검색 중...",
        }
    }

    pub(crate) fn file_search_no_matches(self) -> &'static str {
        match self {
            Self::ZhCn => "没有匹配项",
            Self::ZhTw => "沒有符合項目",
            Self::EnUs => "no matches",
            Self::JaJp => "一致する項目がありません",
            Self::KoKr => "일치하는 항목이 없습니다",
        }
    }

    pub(crate) fn skill_popup_no_matches(self) -> &'static str {
        match self {
            Self::ZhCn => "没有匹配的技能",
            Self::ZhTw => "沒有符合的技能",
            Self::EnUs => "no matching skills",
            Self::JaJp => "一致するスキルがありません",
            Self::KoKr => "일치하는 기술이 없습니다",
        }
    }

    pub(crate) fn vim_mode_message(self, enabled: bool) -> &'static str {
        match (self, enabled) {
            (Self::ZhCn, true) => "已启用 Vim 编辑模式",
            (Self::ZhCn, false) => "已关闭 Vim 编辑模式",
            (Self::ZhTw, true) => "已啟用 Vim 編輯模式",
            (Self::ZhTw, false) => "已關閉 Vim 編輯模式",
            (Self::EnUs, true) => "Vim composer mode enabled",
            (Self::EnUs, false) => "Vim composer mode disabled",
            (Self::JaJp, true) => "Vim 編集モードを有効にしました",
            (Self::JaJp, false) => "Vim 編集モードを無効にしました",
            (Self::KoKr, true) => "Vim 편집 모드를 켰습니다",
            (Self::KoKr, false) => "Vim 편집 모드를 껐습니다",
        }
    }

    pub(crate) fn slash_command_description(self, command: SlashCommand) -> &'static str {
        match (self, command) {
            (Self::ZhCn, SlashCommand::Model) => "选择模型",
            (Self::ZhTw, SlashCommand::Model) => "選擇模型",
            (Self::EnUs, SlashCommand::Model) => "choose a model",
            (Self::JaJp, SlashCommand::Model) => "モデルを選択",
            (Self::KoKr, SlashCommand::Model) => "모델 선택",
            (Self::ZhCn, SlashCommand::Plan) => "切换到计划模式",
            (Self::ZhTw, SlashCommand::Plan) => "切換至計畫模式",
            (Self::EnUs, SlashCommand::Plan) => "switch to Plan mode",
            (Self::JaJp, SlashCommand::Plan) => "計画モードに切り替え",
            (Self::KoKr, SlashCommand::Plan) => "계획 모드로 전환",
            (Self::ZhCn, SlashCommand::Effort) => "设置推理强度",
            (Self::ZhTw, SlashCommand::Effort) => "設定推理強度",
            (Self::EnUs, SlashCommand::Effort) => "set the reasoning effort",
            (Self::JaJp, SlashCommand::Effort) => "推論強度を設定",
            (Self::KoKr, SlashCommand::Effort) => "추론 강도 설정",
            (Self::ZhCn, SlashCommand::Permissions) => "设置权限配置",
            (Self::ZhTw, SlashCommand::Permissions) => "設定權限設定檔",
            (Self::EnUs, SlashCommand::Permissions) => "set the permission profile",
            (Self::JaJp, SlashCommand::Permissions) => "権限プロファイルを設定",
            (Self::KoKr, SlashCommand::Permissions) => "권한 프로필 설정",
            (Self::ZhCn, SlashCommand::Status) => "查看当前会话状态",
            (Self::ZhTw, SlashCommand::Status) => "檢視目前工作階段狀態",
            (Self::EnUs, SlashCommand::Status) => "show the current session status",
            (Self::JaJp, SlashCommand::Status) => "現在のセッション状態を表示",
            (Self::KoKr, SlashCommand::Status) => "현재 세션 상태 보기",
            (Self::ZhCn, SlashCommand::Copy) => "复制最后一条回复",
            (Self::ZhTw, SlashCommand::Copy) => "複製最後一則回覆",
            (Self::EnUs, SlashCommand::Copy) => "copy the last response",
            (Self::JaJp, SlashCommand::Copy) => "最後の応答をコピー",
            (Self::KoKr, SlashCommand::Copy) => "마지막 응답 복사",
            (Self::ZhCn, SlashCommand::Export) => "导出完整对话",
            (Self::ZhTw, SlashCommand::Export) => "匯出完整對話",
            (Self::EnUs, SlashCommand::Export) => "export the complete conversation",
            (Self::JaJp, SlashCommand::Export) => "完全な会話をエクスポート",
            (Self::KoKr, SlashCommand::Export) => "전체 대화 내보내기",
            (Self::ZhCn, SlashCommand::Agents) => "查看和切换所有活跃 Agent 会话",
            (Self::ZhTw, SlashCommand::Agents) => "檢視和切換所有活躍 Agent 工作階段",
            (Self::EnUs, SlashCommand::Agents) => {
                "view and switch between all active agent sessions"
            }
            (Self::JaJp, SlashCommand::Agents) => {
                "すべてのアクティブな Agent セッションを表示して切り替え"
            }
            (Self::KoKr, SlashCommand::Agents) => "모든 활성 에이전트 세션 보기 및 전환",
            (Self::ZhCn, SlashCommand::MultiAgents) => "切换子 Agent 会话",
            (Self::ZhTw, SlashCommand::MultiAgents) => "切換子 Agent 工作階段",
            (Self::EnUs, SlashCommand::MultiAgents) => "switch between sub-agent threads",
            (Self::JaJp, SlashCommand::MultiAgents) => "サブ Agent スレッドを切り替え",
            (Self::KoKr, SlashCommand::MultiAgents) => "하위 에이전트 스레드 전환",
            (Self::ZhCn, SlashCommand::Resume) => "恢复之前的会话",
            (Self::ZhTw, SlashCommand::Resume) => "恢復先前的工作階段",
            (Self::EnUs, SlashCommand::Resume) => "resume a previous session",
            (Self::JaJp, SlashCommand::Resume) => "以前のセッションを再開",
            (Self::KoKr, SlashCommand::Resume) => "이전 세션 재개",
            (Self::ZhCn, SlashCommand::Vim) => "切换 Vim 编辑模式",
            (Self::ZhTw, SlashCommand::Vim) => "切換 Vim 編輯模式",
            (Self::EnUs, SlashCommand::Vim) => "toggle Vim composer mode",
            (Self::JaJp, SlashCommand::Vim) => "Vim 編集モードを切り替え",
            (Self::KoKr, SlashCommand::Vim) => "Vim 편집 모드 전환",
            (Self::ZhCn, SlashCommand::Pwd) => "显示当前工作目录",
            (Self::ZhTw, SlashCommand::Pwd) => "顯示目前工作目錄",
            (Self::EnUs, SlashCommand::Pwd) => "show the current working directory",
            (Self::JaJp, SlashCommand::Pwd) => "現在の作業ディレクトリを表示",
            (Self::KoKr, SlashCommand::Pwd) => "현재 작업 디렉터리 표시",
        }
    }

    pub(crate) fn current_working_directory_message(self, cwd: &str) -> String {
        match self {
            Self::ZhCn => format!("当前工作目录：{cwd}"),
            Self::ZhTw => format!("目前工作目錄：{cwd}"),
            Self::EnUs => format!("Current working directory: {cwd}"),
            Self::JaJp => format!("現在の作業ディレクトリ：{cwd}"),
            Self::KoKr => format!("현재 작업 디렉터리: {cwd}"),
        }
    }

    pub(crate) fn pwd_usage(self) -> &'static str {
        match self {
            Self::ZhCn => "用法：/pwd",
            Self::ZhTw => "用法：/pwd",
            Self::EnUs => "Usage: /pwd",
            Self::JaJp => "使い方：/pwd",
            Self::KoKr => "사용법: /pwd",
        }
    }

    pub(crate) fn skipped_skills_message(self, count: usize) -> String {
        match self {
            Self::ZhCn => format!("由于 SKILL.md 无效，已跳过加载 {count} 个技能。"),
            Self::ZhTw => format!("由於 SKILL.md 無效，已略過載入 {count} 個技能。"),
            Self::EnUs => {
                format!("Skipped loading {count} skill(s) due to invalid SKILL.md files.")
            }
            Self::JaJp => {
                format!("無効な SKILL.md のため、{count} 件のスキルをスキップしました。")
            }
            Self::KoKr => format!("잘못된 SKILL.md로 인해 기술 {count}개를 건너뛰었습니다."),
        }
    }

    pub(crate) fn skill_load_error_message(self, path: &str, message: &str) -> String {
        match self {
            Self::ZhCn => format!("技能加载错误：{path}：{message}"),
            Self::ZhTw => format!("技能載入錯誤：{path}：{message}"),
            Self::EnUs => format!("{path}: {message}"),
            Self::JaJp => format!("スキル読み込みエラー：{path}：{message}"),
            Self::KoKr => format!("기술 로드 오류: {path}: {message}"),
        }
    }

    pub(crate) fn status_title(self) -> &'static str {
        match self {
            Self::ZhCn => "状态",
            Self::ZhTw => "狀態",
            Self::EnUs => "STATUS",
            Self::JaJp => "状態",
            Self::KoKr => "상태",
        }
    }

    pub(crate) fn transcript_title(self) -> &'static str {
        match self {
            Self::ZhCn => "对话记录",
            Self::ZhTw => "對話記錄",
            Self::EnUs => "T R A N S C R I P T",
            Self::JaJp => "会話履歴",
            Self::KoKr => "대화 기록",
        }
    }

    pub(crate) fn export_title(self) -> &'static str {
        match self {
            Self::ZhCn => "导出对话",
            Self::ZhTw => "匯出對話",
            Self::EnUs => "Export conversation",
            Self::JaJp => "会話をエクスポート",
            Self::KoKr => "대화 내보내기",
        }
    }

    pub(crate) fn export_subtitle(self) -> &'static str {
        match self {
            Self::ZhCn => "将完整对话保存为 Markdown",
            Self::ZhTw => "將完整對話儲存為 Markdown",
            Self::EnUs => "Save the complete conversation as Markdown",
            Self::JaJp => "完全な会話を Markdown として保存",
            Self::KoKr => "전체 대화를 Markdown으로 저장",
        }
    }

    pub(crate) fn export_copy_label(self) -> &'static str {
        match self {
            Self::ZhCn => "复制到剪贴板",
            Self::ZhTw => "複製到剪貼簿",
            Self::EnUs => "Copy to clipboard",
            Self::JaJp => "クリップボードにコピー",
            Self::KoKr => "클립보드에 복사",
        }
    }

    pub(crate) fn export_copy_description(self) -> &'static str {
        match self {
            Self::ZhCn => "复制完整 Markdown 对话记录",
            Self::ZhTw => "複製完整 Markdown 對話記錄",
            Self::EnUs => "Copy the complete Markdown transcript",
            Self::JaJp => "完全な Markdown 会話履歴をコピー",
            Self::KoKr => "전체 Markdown 대화 기록 복사",
        }
    }

    pub(crate) fn export_file_label(self) -> &'static str {
        match self {
            Self::ZhCn => "保存到文件",
            Self::ZhTw => "儲存到檔案",
            Self::EnUs => "Save to file",
            Self::JaJp => "ファイルに保存",
            Self::KoKr => "파일에 저장",
        }
    }

    pub(crate) fn export_file_description(self) -> &'static str {
        match self {
            Self::ZhCn => "选择 Markdown 文件名",
            Self::ZhTw => "選擇 Markdown 檔名",
            Self::EnUs => "Choose a Markdown filename",
            Self::JaJp => "Markdown ファイル名を選択",
            Self::KoKr => "Markdown 파일 이름 선택",
        }
    }

    pub(crate) fn export_picker_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "按 Enter 确认，按 Esc 返回",
            Self::ZhTw => "按 Enter 確認，按 Esc 返回",
            Self::EnUs => "Press enter to confirm or esc to go back",
            Self::JaJp => "Enter で確定、Esc で戻る",
            Self::KoKr => "Enter로 확인하거나 Esc로 돌아가기",
        }
    }

    pub(crate) fn export_prompt_title(self) -> &'static str {
        match self {
            Self::ZhCn => "保存对话",
            Self::ZhTw => "儲存對話",
            Self::EnUs => "Save conversation",
            Self::JaJp => "会話を保存",
            Self::KoKr => "대화 저장",
        }
    }

    pub(crate) fn export_prompt_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "输入文件名并按 Enter 确认，按 Esc 返回",
            Self::ZhTw => "輸入檔名並按 Enter 確認，按 Esc 返回",
            Self::EnUs => "Type a filename and press Enter to confirm, Esc to go back",
            Self::JaJp => "ファイル名を入力して Enter で確定、Esc で戻る",
            Self::KoKr => "파일 이름을 입력하고 Enter로 확인하거나 Esc로 돌아가기",
        }
    }

    pub(crate) fn thread_label(self) -> &'static str {
        match self {
            Self::ZhCn => "会话",
            Self::ZhTw => "工作階段",
            Self::EnUs => "thread",
            Self::JaJp => "スレッド",
            Self::KoKr => "스레드",
        }
    }

    pub(crate) fn provider_label(self) -> &'static str {
        match self {
            Self::ZhCn | Self::ZhTw => "Provider",
            Self::EnUs => "provider",
            Self::JaJp => "プロバイダー",
            Self::KoKr => "프로바이더",
        }
    }

    pub(crate) fn cwd_label(self) -> &'static str {
        match self {
            Self::ZhCn => "工作目录",
            Self::ZhTw => "工作目錄",
            Self::EnUs => "working directory",
            Self::JaJp => "作業ディレクトリ",
            Self::KoKr => "작업 디렉터리",
        }
    }

    pub(crate) fn state_label(self) -> &'static str {
        match self {
            Self::ZhCn => "状态",
            Self::ZhTw => "狀態",
            Self::EnUs => "status",
            Self::JaJp => "状態",
            Self::KoKr => "상태",
        }
    }

    pub(crate) fn not_set_label(self) -> &'static str {
        match self {
            Self::ZhCn => "未设置",
            Self::ZhTw => "未設定",
            Self::EnUs => "not set",
            Self::JaJp => "未設定",
            Self::KoKr => "설정되지 않음",
        }
    }

    pub(crate) fn image_label(self) -> &'static str {
        match self {
            Self::ZhCn => "图片",
            Self::ZhTw => "圖片",
            Self::EnUs => "image",
            Self::JaJp => "画像",
            Self::KoKr => "이미지",
        }
    }

    pub(crate) fn numbered_image_label(self, index: &str) -> String {
        let label = match self {
            Self::EnUs => "Image",
            _ => self.image_label(),
        };
        format!("[{label} #{index}]")
    }

    pub(crate) fn edit_queued_input_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "Alt+Up 编辑最后一条排队输入",
            Self::ZhTw => "Alt+Up 編輯最後一則排隊輸入",
            Self::EnUs => "Alt+Up edit last queued input",
            Self::JaJp => "Alt+Up 最後のキュー入力を編集",
            Self::KoKr => "Alt+Up 마지막 대기 입력 편집",
        }
    }

    pub(crate) fn pager_footer(self) -> &'static str {
        match self {
            Self::ZhCn => "上下滚动  PgUp/PgDn 翻页  Home/End 跳转  Esc/Q 关闭",
            Self::ZhTw => "上下捲動  PgUp/PgDn 翻頁  Home/End 跳轉  Esc/Q 關閉",
            Self::EnUs => "Up/Down scroll  PgUp/PgDn page  Home/End jump  Esc/Q close",
            Self::JaJp => "上下スクロール  PgUp/PgDn ページ  Home/End 移動  Esc/Q 閉じる",
            Self::KoKr => "위/아래 스크롤  PgUp/PgDn 페이지  Home/End 이동  Esc/Q 닫기",
        }
    }

    pub(crate) fn transcript_pager_footer(self) -> &'static str {
        match self {
            Self::ZhCn => "上下滚动  PgUp/PgDn 翻页  Home/End 跳转  Ctrl+T/Esc/Q 关闭",
            Self::ZhTw => "上下捲動  PgUp/PgDn 翻頁  Home/End 跳轉  Ctrl+T/Esc/Q 關閉",
            Self::EnUs => "Up/Down scroll  PgUp/PgDn page  Home/End jump  Ctrl+T/Esc/Q close",
            Self::JaJp => "上下スクロール  PgUp/PgDn ページ  Home/End 移動  Ctrl+T/Esc/Q 閉じる",
            Self::KoKr => "위/아래 스크롤  PgUp/PgDn 페이지  Home/End 이동  Ctrl+T/Esc/Q 닫기",
        }
    }

    pub(crate) fn status(self, status: &str) -> String {
        let (prefix, rest) = status
            .split_once(':')
            .map_or((status, None), |(prefix, rest)| {
                (prefix, Some(rest.trim_start()))
            });
        let label = match prefix {
            "" => return String::new(),
            "ready" => self.ready_label(),
            "running" => match self {
                Self::ZhCn => "运行中",
                Self::ZhTw => "執行中",
                Self::EnUs => "running",
                Self::JaJp => "実行中",
                Self::KoKr => "실행 중",
            },
            "running hook" => match self {
                Self::ZhCn => "正在运行 Hook",
                Self::ZhTw => "正在執行 Hook",
                Self::EnUs => "running hook",
                Self::JaJp => "Hook を実行中",
                Self::KoKr => "Hook 실행 중",
            },
            "running hooks" => match self {
                Self::ZhCn => "正在运行多个 Hook",
                Self::ZhTw => "正在執行多個 Hook",
                Self::EnUs => "running hooks",
                Self::JaJp => "複数の Hook を実行中",
                Self::KoKr => "여러 Hook 실행 중",
            },
            "MCP startup issue" => match self {
                Self::ZhCn => "MCP 启动问题",
                Self::ZhTw => "MCP 啟動問題",
                Self::EnUs => "MCP startup issue",
                Self::JaJp => "MCP 起動の問題",
                Self::KoKr => "MCP 시작 문제",
            },
            "MCP startup issues" => match self {
                Self::ZhCn => "MCP 启动问题",
                Self::ZhTw => "MCP 啟動問題",
                Self::EnUs => "MCP startup issues",
                Self::JaJp => "MCP 起動の問題",
                Self::KoKr => "MCP 시작 문제",
            },
            "completed" => match self {
                Self::ZhCn | Self::ZhTw => "已完成",
                Self::EnUs => "completed",
                Self::JaJp => "完了",
                Self::KoKr => "완료",
            },
            "failed" => match self {
                Self::ZhCn => "失败",
                Self::ZhTw => "失敗",
                Self::EnUs => "failed",
                Self::JaJp => "失敗",
                Self::KoKr => "실패",
            },
            "closed" => match self {
                Self::ZhCn => "已关闭",
                Self::ZhTw => "已關閉",
                Self::EnUs => "closed",
                Self::JaJp => "終了",
                Self::KoKr => "닫힘",
            },
            "interrupted" => match self {
                Self::ZhCn => "已中断",
                Self::ZhTw => "已中斷",
                Self::EnUs => "interrupted",
                Self::JaJp => "中断",
                Self::KoKr => "중단됨",
            },
            "background task started" => match self {
                Self::ZhCn => "后台任务已启动",
                Self::ZhTw => "背景工作已啟動",
                Self::EnUs => "background task started",
                Self::JaJp => "バックグラウンドタスクを開始しました",
                Self::KoKr => "백그라운드 작업 시작됨",
            },
            "background task failed" => match self {
                Self::ZhCn => "后台任务失败",
                Self::ZhTw => "背景工作失敗",
                Self::EnUs => "background task failed",
                Self::JaJp => "バックグラウンドタスクに失敗しました",
                Self::KoKr => "백그라운드 작업 실패",
            },
            "background task is idle" => match self {
                Self::ZhCn => "后台任务未运行",
                Self::ZhTw => "背景工作未執行",
                Self::EnUs => "background task is idle",
                Self::JaJp => "バックグラウンドタスクは待機中です",
                Self::KoKr => "백그라운드 작업 대기 중",
            },
            "stopping background turn" => match self {
                Self::ZhCn => "正在停止后台回合",
                Self::ZhTw => "正在停止背景回合",
                Self::EnUs => "stopping background turn",
                Self::JaJp => "バックグラウンドターンを停止中",
                Self::KoKr => "백그라운드 턴 중지 중",
            },
            "agent renamed" => match self {
                Self::ZhCn => "Agent 已改名",
                Self::ZhTw => "Agent 已改名",
                Self::EnUs => "agent renamed",
                Self::JaJp => "Agent の名前を変更しました",
                Self::KoKr => "에이전트 이름 변경됨",
            },
            "resume picker is available from /resume" => match self {
                Self::ZhCn => "请使用 /resume 打开会话选择器",
                Self::ZhTw => "請使用 /resume 開啟工作階段選擇器",
                Self::EnUs => "resume picker is available from /resume",
                Self::JaJp => "/resume から再開ピッカーを開けます",
                Self::KoKr => "/resume에서 재개 선택기를 열 수 있음",
            },
            "declined" => match self {
                Self::ZhCn => "已拒绝",
                Self::ZhTw => "已拒絕",
                Self::EnUs => "declined",
                Self::JaJp => "拒否",
                Self::KoKr => "거부됨",
            },
            "settings updated" => match self {
                Self::ZhCn => "设置已更新",
                Self::ZhTw => "設定已更新",
                Self::EnUs => "settings updated",
                Self::JaJp => "設定を更新しました",
                Self::KoKr => "설정이 업데이트됨",
            },
            "collaboration mode updated" => match self {
                Self::ZhCn => "协作模式已更新",
                Self::ZhTw => "協作模式已更新",
                Self::EnUs => "collaboration mode updated",
                Self::JaJp => "コラボレーションモードを更新しました",
                Self::KoKr => "협업 모드가 업데이트됨",
            },
            "plan mode" => match self {
                Self::ZhCn => "计划模式",
                Self::ZhTw => "計畫模式",
                Self::EnUs => "plan mode",
                Self::JaJp => "計画モード",
                Self::KoKr => "계획 모드",
            },
            "plan mode unavailable on this server" => match self {
                Self::ZhCn => "此服务器不支持计划模式",
                Self::ZhTw => "此伺服器不支援計畫模式",
                Self::EnUs => "plan mode unavailable on this server",
                Self::JaJp => "このサーバーでは計画モードを利用できません",
                Self::KoKr => "이 서버에서는 계획 모드를 사용할 수 없음",
            },
            "switched agent" => match self {
                Self::ZhCn => "已切换 Agent",
                Self::ZhTw => "已切換 Agent",
                Self::EnUs => "switched agent",
                Self::JaJp => "Agent を切り替えました",
                Self::KoKr => "에이전트 전환됨",
            },
            "sub-agent thread is parent-owned" => match self {
                Self::ZhCn => "子 Agent 会话由父会话控制",
                Self::ZhTw => "子 Agent 工作階段由父工作階段控制",
                Self::EnUs => "sub-agent thread is parent-owned",
                Self::JaJp => "サブ Agent スレッドは親が管理します",
                Self::KoKr => "하위 에이전트 스레드는 부모가 제어함",
            },
            "agent switch failed" => match self {
                Self::ZhCn => "切换 Agent 失败",
                Self::ZhTw => "切換 Agent 失敗",
                Self::EnUs => "agent switch failed",
                Self::JaJp => "Agent の切り替えに失敗しました",
                Self::KoKr => "에이전트 전환 실패",
            },
            "no sub-agents available" => match self {
                Self::ZhCn => "暂无可用的子 Agent",
                Self::ZhTw => "目前沒有可用的子 Agent",
                Self::EnUs => "no sub-agents available",
                Self::JaJp => "利用可能なサブ Agent がありません",
                Self::KoKr => "사용 가능한 하위 에이전트가 없음",
            },
            "choose model" => match self {
                Self::ZhCn => "请选择模型",
                Self::ZhTw => "請選擇模型",
                Self::EnUs => "choose model",
                Self::JaJp => "モデルを選択",
                Self::KoKr => "모델 선택",
            },
            "steering" => match self {
                Self::ZhCn => "正在引导",
                Self::ZhTw => "正在引導",
                Self::EnUs => "steering",
                Self::JaJp => "ステアリング中",
                Self::KoKr => "조정 중",
            },
            "queued" => match self {
                Self::ZhCn => "已排队",
                Self::ZhTw => "已排隊",
                Self::EnUs => "queued",
                Self::JaJp => "キューに追加済み",
                Self::KoKr => "대기열에 추가됨",
            },
            "queue unavailable" => match self {
                Self::ZhCn => "队列不可用",
                Self::ZhTw => "佇列無法使用",
                Self::EnUs => "queue unavailable",
                Self::JaJp => "キューを利用できません",
                Self::KoKr => "대기열을 사용할 수 없음",
            },
            "queued input editing" => match self {
                Self::ZhCn => "正在编辑排队输入",
                Self::ZhTw => "正在編輯排隊輸入",
                Self::EnUs => "editing queued input",
                Self::JaJp => "キュー入力を編集中",
                Self::KoKr => "대기 입력 편집 중",
            },
            "queued input unavailable" => match self {
                Self::ZhCn => "排队输入已不可用",
                Self::ZhTw => "排隊輸入已無法使用",
                Self::EnUs => "queued input unavailable",
                Self::JaJp => "キュー入力を利用できません",
                Self::KoKr => "대기 입력을 사용할 수 없음",
            },
            "queue edit failed" => match self {
                Self::ZhCn => "编辑排队输入失败",
                Self::ZhTw => "編輯排隊輸入失敗",
                Self::EnUs => "queue edit failed",
                Self::JaJp => "キュー入力の編集に失敗しました",
                Self::KoKr => "대기 입력 편집 실패",
            },
            "interrupting" => match self {
                Self::ZhCn => "正在中断",
                Self::ZhTw => "正在中斷",
                Self::EnUs => "interrupting",
                Self::JaJp => "中断中",
                Self::KoKr => "중단 중",
            },
            "editor draft empty" => match self {
                Self::ZhCn => "编辑器草稿为空",
                Self::ZhTw => "編輯器草稿為空",
                Self::EnUs => "editor draft empty",
                Self::JaJp => "エディターの下書きが空です",
                Self::KoKr => "편집기 초안이 비어 있음",
            },
            "copied last response" => match self {
                Self::ZhCn => "已复制上一条回复",
                Self::ZhTw => "已複製上一則回覆",
                Self::EnUs => "copied last response",
                Self::JaJp => "前回の応答をコピーしました",
                Self::KoKr => "마지막 응답을 복사함",
            },
            "no agent response to copy" => match self {
                Self::ZhCn => "没有可复制的 Agent 回复",
                Self::ZhTw => "沒有可複製的 Agent 回覆",
                Self::EnUs => "no agent response to copy",
                Self::JaJp => "コピーできる Agent 応答がありません",
                Self::KoKr => "복사할 Agent 응답이 없음",
            },
            "copy failed" => match self {
                Self::ZhCn => "复制失败",
                Self::ZhTw => "複製失敗",
                Self::EnUs => "copy failed",
                Self::JaJp => "コピーに失敗しました",
                Self::KoKr => "복사 실패",
            },
            "exported conversation to clipboard" => match self {
                Self::ZhCn => "已将完整对话复制到剪贴板",
                Self::ZhTw => "已將完整對話複製到剪貼簿",
                Self::EnUs => "exported conversation to clipboard",
                Self::JaJp => "完全な会話をクリップボードにコピーしました",
                Self::KoKr => "전체 대화를 클립보드에 복사함",
            },
            "exported conversation to" => match self {
                Self::ZhCn => "完整对话已导出到",
                Self::ZhTw => "完整對話已匯出到",
                Self::EnUs => "exported conversation to",
                Self::JaJp => "完全な会話をエクスポートしました",
                Self::KoKr => "전체 대화를 내보냄",
            },
            "export failed" => match self {
                Self::ZhCn => "导出失败",
                Self::ZhTw => "匯出失敗",
                Self::EnUs => "export failed",
                Self::JaJp => "エクスポートに失敗しました",
                Self::KoKr => "내보내기 실패",
            },
            "image attached" => match self {
                Self::ZhCn => "已附加图片",
                Self::ZhTw => "已附加圖片",
                Self::EnUs => "image attached",
                Self::JaJp => "画像を添付しました",
                Self::KoKr => "이미지 첨부됨",
            },
            "image paste failed" => match self {
                Self::ZhCn => "粘贴图片失败",
                Self::ZhTw => "貼上圖片失敗",
                Self::EnUs => "image paste failed",
                Self::JaJp => "画像の貼り付けに失敗しました",
                Self::KoKr => "이미지 붙여넣기 실패",
            },
            "reconnecting" => match self {
                Self::ZhCn => "正在重连",
                Self::ZhTw => "正在重新連線",
                Self::EnUs => "reconnecting",
                Self::JaJp => "再接続中",
                Self::KoKr => "재연결 중",
            },
            "effort" => self.effort_label(),
            "permissions" => self.permissions_label(),
            "prompt history unavailable" => match self {
                Self::ZhCn => "提示历史不可用",
                Self::ZhTw => "提示歷史不可用",
                Self::EnUs => "prompt history unavailable",
                Self::JaJp => "プロンプト履歴を利用できません",
                Self::KoKr => "프롬프트 기록을 사용할 수 없음",
            },
            "editor unavailable" => match self {
                Self::ZhCn => "编辑器不可用",
                Self::ZhTw => "編輯器不可用",
                Self::EnUs => "editor unavailable",
                Self::JaJp => "エディターを利用できません",
                Self::KoKr => "편집기를 사용할 수 없음",
            },
            _ => return status.to_string(),
        };
        rest.map_or_else(|| label.to_string(), |rest| format!("{label}: {rest}"))
    }

    pub(crate) fn multi_agent(self, text: &str) -> String {
        if text == "Finished waiting" {
            return match self {
                Self::ZhCn => "等待完成".to_string(),
                Self::ZhTw => "等待完成".to_string(),
                Self::EnUs => text.to_string(),
                Self::JaJp => "待機が完了しました".to_string(),
                Self::KoKr => "대기 완료".to_string(),
            };
        }

        const PREFIXES: &[(&str, &str, &str, &str, &str)] = &[
            ("Spawned ", "已启动 ", "已啟動 ", "起動: ", "생성됨 "),
            (
                "Sent input to ",
                "已向其发送输入 ",
                "已向其傳送輸入 ",
                "入力送信: ",
                "입력 전송: ",
            ),
            (
                "Resuming ",
                "正在恢复 ",
                "正在恢復 ",
                "再開中: ",
                "재개 중: ",
            ),
            ("Resumed ", "已恢复 ", "已恢復 ", "再開済み: ", "재개됨 "),
            (
                "Waiting for ",
                "正在等待 ",
                "正在等待 ",
                "待機中: ",
                "대기 중: ",
            ),
            ("Closed ", "已关闭 ", "已關閉 ", "終了: ", "닫힘 "),
            ("Started ", "已开始 ", "已開始 ", "開始: ", "시작됨 "),
            (
                "Interacted with ",
                "已与其交互 ",
                "已與其互動 ",
                "操作: ",
                "상호작용: ",
            ),
            ("Interrupted ", "已中断 ", "已中斷 ", "中断: ", "중단됨 "),
        ];
        PREFIXES
            .iter()
            .find_map(|(prefix, zh_cn, zh_tw, ja, ko)| {
                text.strip_prefix(prefix).map(|rest| {
                    let label = match self {
                        Self::ZhCn => zh_cn,
                        Self::ZhTw => zh_tw,
                        Self::EnUs => prefix,
                        Self::JaJp => ja,
                        Self::KoKr => ko,
                    };
                    format!("{label}{rest}")
                })
            })
            .unwrap_or_else(|| text.to_string())
    }

    pub(crate) fn detail(self, detail: &str) -> String {
        const PREFIXES: &[(&str, &str, &str, &str, &str)] = &[
            ("exit ", "退出 ", "退出 ", "exit ", "종료 "),
            ("duration ", "耗时 ", "耗時 ", "所要時間 ", "소요 시간 "),
            (
                "files: ",
                "文件数：",
                "檔案數：",
                "ファイル数: ",
                "파일 수: ",
            ),
            ("added: ", "新增：", "新增：", "追加: ", "추가: "),
            ("deleted: ", "删除：", "刪除：", "削除: ", "삭제: "),
            ("updated: ", "更新：", "更新：", "更新: ", "업데이트: "),
            (
                "result items: ",
                "结果项：",
                "結果項：",
                "結果項目: ",
                "결과 항목: ",
            ),
            (
                "content types: ",
                "内容类型：",
                "內容類型：",
                "コンテンツ種別: ",
                "콘텐츠 유형: ",
            ),
            ("output: ", "输出：", "輸出：", "出力: ", "출력: "),
            (
                "computer action: ",
                "电脑操作：",
                "電腦操作：",
                "コンピュータ操作: ",
                "컴퓨터 작업: ",
            ),
            (
                "computer error: ",
                "电脑错误：",
                "電腦錯誤：",
                "コンピュータ エラー: ",
                "컴퓨터 오류: ",
            ),
            (
                "computer screenshot: ",
                "电脑截图：",
                "電腦螢幕截圖：",
                "コンピュータ スクリーンショット: ",
                "컴퓨터 스크린샷: ",
            ),
            (
                "structured content: ",
                "结构化内容：",
                "結構化內容：",
                "構造化コンテンツ: ",
                "구조화 콘텐츠: ",
            ),
            ("truncated", "已截断", "已截斷", "切り詰め済み", "잘림"),
            (
                "output available",
                "可获取输出",
                "可取得輸出",
                "出力を取得可能",
                "출력 사용 가능",
            ),
            ("error: ", "错误：", "錯誤：", "エラー: ", "오류: "),
            ("success: ", "成功：", "成功：", "成功: ", "성공: "),
            (
                "content items: ",
                "内容项：",
                "內容項：",
                "内容項目: ",
                "콘텐츠 항목: ",
            ),
            (
                "agents: ",
                "Agent 数：",
                "Agent 數：",
                "Agent 数: ",
                "에이전트 수: ",
            ),
            ("model: ", "模型：", "模型：", "モデル: ", "모델: "),
            (
                "effort: ",
                "思考强度：",
                "思考強度：",
                "推論強度: ",
                "추론 강도: ",
            ),
            (
                "prompt: ",
                "提示词：",
                "提示詞：",
                "プロンプト: ",
                "프롬프트: ",
            ),
            (
                "web search: ",
                "网页搜索：",
                "網頁搜尋：",
                "ウェブ検索: ",
                "웹 검색: ",
            ),
            (
                "hook completed",
                "Hook 已完成",
                "Hook 已完成",
                "Hook 完了",
                "Hook 완료",
            ),
            (
                "hook failed",
                "Hook 失败",
                "Hook 失敗",
                "Hook 失敗",
                "Hook 실패",
            ),
            (
                "hook blocked",
                "Hook 已阻断",
                "Hook 已封鎖",
                "Hook がブロックされました",
                "Hook 차단됨",
            ),
            (
                "hook stopped",
                "Hook 已停止",
                "Hook 已停止",
                "Hook 停止",
                "Hook 중지됨",
            ),
            (
                "hook output: ",
                "Hook 输出：",
                "Hook 輸出：",
                "Hook 出力: ",
                "Hook 출력: ",
            ),
            (
                "searching the web",
                "正在搜索网页",
                "正在搜尋網頁",
                "ウェブを検索中",
                "웹 검색 중",
            ),
            (
                "searched the web for ",
                "已搜索网页：",
                "已搜尋網頁：",
                "ウェブ検索済み：",
                "웹 검색 완료: ",
            ),
            (
                "searched the web",
                "已搜索网页",
                "已搜尋網頁",
                "ウェブ検索済み",
                "웹 검색 완료",
            ),
            (
                "view image: ",
                "查看图片：",
                "檢視圖片：",
                "画像を表示: ",
                "이미지 보기: ",
            ),
            ("result: ", "结果：", "結果：", "結果: ", "결과: "),
            ("saved: ", "已保存：", "已儲存：", "保存済み: ", "저장됨: "),
            (
                "revised prompt: ",
                "修订提示：",
                "修訂提示：",
                "修正プロンプト: ",
                "수정된 프롬프트: ",
            ),
            (
                "review started: ",
                "审查开始：",
                "審查開始：",
                "レビュー開始: ",
                "검토 시작: ",
            ),
            (
                "review completed: ",
                "审查完成：",
                "審查完成：",
                "レビュー完成: ",
                "검토 완료: ",
            ),
            (
                "context compacted",
                "上下文已压缩",
                "內容已壓縮",
                "コンテキストを圧縮しました",
                "컨텍스트가 압축됨",
            ),
            (
                "image generation",
                "图片生成",
                "圖片生成",
                "画像生成",
                "이미지 생성",
            ),
            (
                "unsupported item: ",
                "不支持的条目：",
                "不支援的項目：",
                "未対応項目: ",
                "지원되지 않는 항목: ",
            ),
            ("pending: ", "待处理：", "待處理：", "保留中: ", "대기 중: "),
            ("running: ", "运行中：", "執行中：", "実行中: ", "실행 중: "),
            (
                "interrupted: ",
                "已中断：",
                "已中斷：",
                "中断: ",
                "중단됨: ",
            ),
            ("completed: ", "已完成：", "已完成：", "完了: ", "완료: "),
            ("errored: ", "出错：", "發生錯誤：", "エラー: ", "오류: "),
            (
                "shutdown: ",
                "已关闭：",
                "已關閉：",
                "シャットダウン: ",
                "종료됨: ",
            ),
            (
                "not-found: ",
                "未找到：",
                "找不到：",
                "見つかりません: ",
                "찾을 수 없음: ",
            ),
        ];
        PREFIXES
            .iter()
            .find_map(|(prefix, zh_cn, zh_tw, ja, ko)| {
                detail.strip_prefix(prefix).map(|rest| {
                    let label = match self {
                        Self::ZhCn => zh_cn,
                        Self::ZhTw => zh_tw,
                        Self::EnUs => prefix,
                        Self::JaJp => ja,
                        Self::KoKr => ko,
                    };
                    format!("{label}{rest}")
                })
            })
            .unwrap_or_else(|| detail.to_string())
    }

    pub(crate) fn output_omitted_lines(self, count: usize) -> String {
        match self {
            Self::ZhCn => format!("… 已省略 {count} 行 …"),
            Self::ZhTw => format!("… 已省略 {count} 行 …"),
            Self::EnUs => format!("… {count} lines omitted …"),
            Self::JaJp => format!("… {count} 行を省略 …"),
            Self::KoKr => format!("… {count}줄 생략 …"),
        }
    }

    pub(crate) fn output_omitted_bytes(self, count: usize) -> String {
        match self {
            Self::ZhCn => format!("… 已省略 {count} 字节 …"),
            Self::ZhTw => format!("… 已省略 {count} 位元組 …"),
            Self::EnUs => format!("… {count} bytes omitted …"),
            Self::JaJp => format!("… {count} バイトを省略 …"),
            Self::KoKr => format!("… {count}바이트 생략 …"),
        }
    }

    pub(crate) fn approval_title(self, kind: &str) -> &'static str {
        match (self, kind) {
            (Self::ZhCn, "command") => "批准命令？",
            (Self::ZhTw, "command") => "核准命令？",
            (Self::JaJp, "command") => "コマンドを承認しますか？",
            (Self::KoKr, "command") => "명령을 승인할까요?",
            (Self::ZhCn, "file") => "批准文件更改？",
            (Self::ZhTw, "file") => "核准檔案變更？",
            (Self::JaJp, "file") => "ファイル変更を承認しますか？",
            (Self::KoKr, "file") => "파일 변경을 승인할까요?",
            (Self::ZhCn, "permissions") => "授予额外权限？",
            (Self::ZhTw, "permissions") => "授予額外權限？",
            (Self::JaJp, "permissions") => "追加権限を付与しますか？",
            (Self::KoKr, "permissions") => "추가 권한을 부여할까요?",
            (_, "command") => "Approve command?",
            (_, "file") => "Approve file changes?",
            _ => "Grant additional permissions?",
        }
    }

    pub(crate) fn approval_option(self, label: &str) -> String {
        match (self, label) {
            (Self::ZhCn, "Accept") => "批准".to_string(),
            (Self::ZhTw, "Accept") => "核准".to_string(),
            (Self::JaJp, "Accept") => "承認".to_string(),
            (Self::KoKr, "Accept") => "승인".to_string(),
            (Self::ZhCn, "Decline") => "拒绝".to_string(),
            (Self::ZhTw, "Decline") => "拒絕".to_string(),
            (Self::JaJp, "Decline") => "拒否".to_string(),
            (Self::KoKr, "Decline") => "거부".to_string(),
            (Self::ZhCn, "Cancel") => "取消".to_string(),
            (Self::ZhTw, "Cancel") => "取消".to_string(),
            (Self::JaJp, "Cancel") => "キャンセル".to_string(),
            (Self::KoKr, "Cancel") => "취소".to_string(),
            (Self::ZhCn, "Allow once") => "允许一次".to_string(),
            (Self::ZhTw, "Allow once") => "允許一次".to_string(),
            (Self::JaJp, "Allow once") => "一度だけ許可".to_string(),
            (Self::KoKr, "Allow once") => "한 번 허용".to_string(),
            (Self::ZhCn, "Allow for this session") => "允许本会话".to_string(),
            (Self::ZhTw, "Allow for this session") => "允許本工作階段".to_string(),
            (Self::JaJp, "Allow for this session") => "このセッションで許可".to_string(),
            (Self::KoKr, "Allow for this session") => "이 세션에 허용".to_string(),
            (Self::ZhCn, "Cancel turn") => "取消本回合".to_string(),
            (Self::ZhTw, "Cancel turn") => "取消本回合".to_string(),
            (Self::JaJp, "Cancel turn") => "ターンをキャンセル".to_string(),
            (Self::KoKr, "Cancel turn") => "턴 취소".to_string(),
            (Self::ZhCn, "Grant for this turn") => "仅授予本回合".to_string(),
            (Self::ZhTw, "Grant for this turn") => "僅授予本回合".to_string(),
            (Self::JaJp, "Grant for this turn") => "このターンのみ許可".to_string(),
            (Self::KoKr, "Grant for this turn") => "이 턴에만 허용".to_string(),
            (Self::ZhCn, "Grant for this session") => "授予本会话".to_string(),
            (Self::ZhTw, "Grant for this session") => "授予本工作階段".to_string(),
            (Self::JaJp, "Grant for this session") => "このセッションで許可".to_string(),
            (Self::KoKr, "Grant for this session") => "이 세션에 허용".to_string(),
            _ => label.to_string(),
        }
    }

    pub(crate) fn mcp_elicitation_title(self, server_name: &str) -> String {
        match self {
            Self::ZhCn => format!("MCP 请求：{server_name}"),
            Self::ZhTw => format!("MCP 要求：{server_name}"),
            Self::EnUs => format!("MCP request: {server_name}"),
            Self::JaJp => format!("MCP リクエスト：{server_name}"),
            Self::KoKr => format!("MCP 요청: {server_name}"),
        }
    }

    pub(crate) fn mcp_elicitation_progress(self, current: usize, total: usize) -> String {
        match self {
            Self::ZhCn => format!("字段 {current}/{total}"),
            Self::ZhTw => format!("欄位 {current}/{total}"),
            Self::EnUs => format!("Field {current}/{total}"),
            Self::JaJp => format!("フィールド {current}/{total}"),
            Self::KoKr => format!("필드 {current}/{total}"),
        }
    }

    pub(crate) fn mcp_elicitation_text_placeholder(self, required: bool) -> &'static str {
        match (self, required) {
            (Self::ZhCn, true) => "请输入答案",
            (Self::ZhCn, false) => "请输入答案（可选）",
            (Self::ZhTw, true) => "請輸入答案",
            (Self::ZhTw, false) => "請輸入答案（選填）",
            (Self::EnUs, true) => "Type an answer",
            (Self::EnUs, false) => "Type an answer (optional)",
            (Self::JaJp, true) => "回答を入力",
            (Self::JaJp, false) => "回答を入力（任意）",
            (Self::KoKr, true) => "답변 입력",
            (Self::KoKr, false) => "답변 입력 (선택 사항)",
        }
    }

    pub(crate) fn mcp_elicitation_boolean_option(self, value: bool) -> &'static str {
        match (self, value) {
            (Self::ZhCn, true) => "是",
            (Self::ZhCn, false) => "否",
            (Self::ZhTw, true) => "是",
            (Self::ZhTw, false) => "否",
            (Self::EnUs, true) => "True",
            (Self::EnUs, false) => "False",
            (Self::JaJp, true) => "はい",
            (Self::JaJp, false) => "いいえ",
            (Self::KoKr, true) => "예",
            (Self::KoKr, false) => "아니요",
        }
    }

    pub(crate) fn mcp_elicitation_approval_option(self, value: &str) -> &'static str {
        match (self, value) {
            (Self::ZhCn, "accept") => "允许",
            (Self::ZhCn, "accept_session") => "允许本次会话",
            (Self::ZhCn, "accept_always") => "始终允许",
            (Self::ZhCn, "decline") => "拒绝",
            (Self::ZhCn, "cancel") => "取消",
            (Self::ZhTw, "accept") => "允許",
            (Self::ZhTw, "accept_session") => "允許此工作階段",
            (Self::ZhTw, "accept_always") => "一律允許",
            (Self::ZhTw, "decline") => "拒絕",
            (Self::ZhTw, "cancel") => "取消",
            (Self::EnUs, "accept") => "Allow",
            (Self::EnUs, "accept_session") => "Allow for this session",
            (Self::EnUs, "accept_always") => "Always allow",
            (Self::EnUs, "decline") => "Deny",
            (Self::EnUs, "cancel") => "Cancel",
            (Self::JaJp, "accept") => "許可",
            (Self::JaJp, "accept_session") => "このセッションで許可",
            (Self::JaJp, "accept_always") => "常に許可",
            (Self::JaJp, "decline") => "拒否",
            (Self::JaJp, "cancel") => "キャンセル",
            (Self::KoKr, "accept") => "허용",
            (Self::KoKr, "accept_session") => "이 세션에서 허용",
            (Self::KoKr, "accept_always") => "항상 허용",
            (Self::KoKr, "decline") => "거부",
            (Self::KoKr, "cancel") => "취소",
            (_, _) => "",
        }
    }

    pub(crate) fn mcp_elicitation_controls(self, select: bool) -> String {
        match (self, select) {
            (Self::ZhCn, true) => "↑/↓ 选择  Enter 确认  Tab/←/→ 切换字段  Esc 取消".to_string(),
            (Self::ZhCn, false) => "Enter 确认  Tab 切换字段  Esc 取消".to_string(),
            (Self::ZhTw, true) => "↑/↓ 選擇  Enter 確認  Tab/←/→ 切換欄位  Esc 取消".to_string(),
            (Self::ZhTw, false) => "Enter 確認  Tab 切換欄位  Esc 取消".to_string(),
            (Self::EnUs, true) => {
                "Up/Down select  Enter confirm  Tab/Left/Right switch field  Esc cancel".to_string()
            }
            (Self::EnUs, false) => "Enter confirm  Tab switch field  Esc cancel".to_string(),
            (Self::JaJp, true) => {
                "↑/↓ 選択  Enter 確定  Tab/←/→ フィールド切替  Esc キャンセル".to_string()
            }
            (Self::JaJp, false) => "Enter 確定  Tab フィールド切替  Esc キャンセル".to_string(),
            (Self::KoKr, true) => "↑/↓ 선택  Enter 확인  Tab/←/→ 필드 전환  Esc 취소".to_string(),
            (Self::KoKr, false) => "Enter 확인  Tab 필드 전환  Esc 취소".to_string(),
        }
    }

    pub(crate) fn mcp_elicitation_required_error(self) -> &'static str {
        match self {
            Self::ZhCn => "请先填写必填字段。",
            Self::ZhTw => "請先填寫必填欄位。",
            Self::EnUs => "Answer all required fields first.",
            Self::JaJp => "必須フィールドを先に入力してください。",
            Self::KoKr => "필수 필드를 먼저 입력하세요.",
        }
    }

    pub(crate) fn mcp_elicitation_invalid(self) -> &'static str {
        match self {
            Self::ZhCn => "无法显示此 MCP 请求",
            Self::ZhTw => "無法顯示此 MCP 要求",
            Self::EnUs => "Unable to display this MCP request",
            Self::JaJp => "この MCP リクエストを表示できません",
            Self::KoKr => "이 MCP 요청을 표시할 수 없습니다",
        }
    }

    pub(crate) fn picker_title(self) -> &'static str {
        match self {
            Self::ZhCn => "选择模型",
            Self::ZhTw => "選擇模型",
            Self::EnUs => "Choose a model",
            Self::JaJp => "モデルを選択",
            Self::KoKr => "모델 선택",
        }
    }

    pub(crate) fn picker_empty(self) -> &'static str {
        match self {
            Self::ZhCn => "没有匹配的模型。按 Esc 取消",
            Self::ZhTw => "沒有符合的模型。按 Esc 取消",
            Self::EnUs => "No matching models. Esc cancel",
            Self::JaJp => "一致するモデルがありません。Esc でキャンセル",
            Self::KoKr => "일치하는 모델이 없습니다. Esc로 취소",
        }
    }

    pub(crate) fn picker_footer(self) -> &'static str {
        match self {
            Self::ZhCn => "上下移动  Enter 选择  Esc 取消",
            Self::ZhTw => "上下移動  Enter 選擇  Esc 取消",
            Self::EnUs => "Up/Down move  Enter select  Esc cancel",
            Self::JaJp => "上下移動  Enter 選択  Esc キャンセル",
            Self::KoKr => "위/아래 이동  Enter 선택  Esc 취소",
        }
    }

    pub(crate) fn agent_picker_title(self) -> &'static str {
        match self {
            Self::ZhCn => "选择 Agent",
            Self::ZhTw => "選擇 Agent",
            Self::EnUs => "Choose an agent",
            Self::JaJp => "Agent を選択",
            Self::KoKr => "에이전트 선택",
        }
    }

    pub(crate) fn agent_picker_empty(self) -> &'static str {
        match self {
            Self::ZhCn => "暂无可用的子 Agent。按 Esc 取消",
            Self::ZhTw => "目前沒有可用的子 Agent。按 Esc 取消",
            Self::EnUs => "No sub-agents available. Esc cancel",
            Self::JaJp => "利用可能なサブ Agent がありません。Esc でキャンセル",
            Self::KoKr => "사용 가능한 하위 에이전트가 없습니다. Esc로 취소",
        }
    }

    pub(crate) fn agent_picker_footer(self) -> &'static str {
        match self {
            Self::ZhCn => "上下移动  Enter 查看  Esc 取消",
            Self::ZhTw => "上下移動  Enter 檢視  Esc 取消",
            Self::EnUs => "Up/Down move  Enter view  Esc cancel",
            Self::JaJp => "上下移動  Enter 表示  Esc キャンセル",
            Self::KoKr => "위/아래 이동  Enter 보기  Esc 취소",
        }
    }

    pub(crate) fn agents_overview_title(self) -> &'static str {
        match self {
            Self::ZhCn => "Agent 控制中心",
            Self::ZhTw => "Agent 控制中心",
            Self::EnUs => "Agent command center",
            Self::JaJp => "Agent コマンドセンター",
            Self::KoKr => "에이전트 명령 센터",
        }
    }

    pub(crate) fn agents_overview_footer(self) -> &'static str {
        match self {
            Self::ZhCn => {
                "上下移动 · Enter 打开 · / 或 Ctrl+F 搜索 · Ctrl+N 新建 · Ctrl+R 改名 · Ctrl+X 停止 · Ctrl+S 分组 · r 刷新 · Esc 返回"
            }
            Self::ZhTw => {
                "上下移動 · Enter 開啟 · / 或 Ctrl+F 搜尋 · Ctrl+N 新建 · Ctrl+R 改名 · Ctrl+X 停止 · Ctrl+S 分組 · r 重新整理 · Esc 返回"
            }
            Self::EnUs => {
                "Up/Down navigate · Enter open · / or Ctrl+F search · Ctrl+N new · Ctrl+R rename · Ctrl+X stop · Ctrl+S group · r refresh · Esc back"
            }
            Self::JaJp => {
                "上下移動 · Enter 開く · / または Ctrl+F 検索 · Ctrl+N 新規 · Ctrl+R 名前変更 · Ctrl+X 停止 · Ctrl+S グループ · r 更新 · Esc 戻る"
            }
            Self::KoKr => {
                "위/아래 이동 · Enter 열기 · / 또는 Ctrl+F 검색 · Ctrl+N 새 작업 · Ctrl+R 이름 변경 · Ctrl+X 중지 · Ctrl+S 그룹 · r 새로 고침 · Esc 뒤로"
            }
        }
    }

    pub(crate) fn agents_overview_input_prefix(self, rename: bool) -> &'static str {
        match (self, rename) {
            (Self::ZhCn, true) => "改名",
            (Self::ZhCn, false) => "新建任务",
            (Self::ZhTw, true) => "改名",
            (Self::ZhTw, false) => "新建工作",
            (Self::EnUs, true) => "Rename",
            (Self::EnUs, false) => "New task",
            (Self::JaJp, true) => "名前変更",
            (Self::JaJp, false) => "新しいタスク",
            (Self::KoKr, true) => "이름 변경",
            (Self::KoKr, false) => "새 작업",
        }
    }

    pub(crate) fn agents_overview_search_prefix(self) -> &'static str {
        match self {
            Self::ZhCn => "搜索",
            Self::ZhTw => "搜尋",
            Self::EnUs => "Search",
            Self::JaJp => "検索",
            Self::KoKr => "검색",
        }
    }

    pub(crate) fn agents_overview_group_label(self, group: &'static str) -> &'static str {
        match (self, group) {
            (Self::ZhCn, "need input") => "需要输入",
            (Self::ZhTw, "need input") => "需要輸入",
            (Self::JaJp, "need input") => "入力待ち",
            (Self::KoKr, "need input") => "입력 필요",
            (Self::ZhCn, "working") => "运行中",
            (Self::ZhTw, "working") => "執行中",
            (Self::JaJp, "working") => "実行中",
            (Self::KoKr, "working") => "작업 중",
            (Self::ZhCn, "ready") => "就绪",
            (Self::ZhTw, "ready") => "就緒",
            (Self::JaJp, "ready") => "準備完了",
            (Self::KoKr, "ready") => "준비됨",
            (Self::ZhCn, "finished") => "已完成",
            (Self::ZhTw, "finished") => "已完成",
            (Self::JaJp, "finished") => "完了",
            (Self::KoKr, "finished") => "완료",
            (_, value) => value,
        }
    }

    pub(crate) fn resume_title(self) -> &'static str {
        match self {
            Self::ZhCn => "选择会话",
            Self::ZhTw => "選擇工作階段",
            Self::EnUs => "Select a conversation",
            Self::JaJp => "会話を選択",
            Self::KoKr => "대화 선택",
        }
    }

    pub(crate) fn untitled_conversation(self) -> &'static str {
        match self {
            Self::ZhCn => "未命名会话",
            Self::ZhTw => "未命名工作階段",
            Self::EnUs => "Untitled conversation",
            Self::JaJp => "無題の会話",
            Self::KoKr => "제목 없는 대화",
        }
    }

    pub(crate) fn resume_label(self) -> &'static str {
        match self {
            Self::ZhCn => "恢复",
            Self::ZhTw => "恢復",
            Self::EnUs => "Resume",
            Self::JaJp => "再開",
            Self::KoKr => "재개",
        }
    }

    pub(crate) fn thread_state(self, state: &str) -> Cow<'static, str> {
        match (self, state) {
            (Self::ZhCn, "active") => Cow::Borrowed("活动"),
            (Self::ZhTw, "active") => Cow::Borrowed("使用中"),
            (Self::JaJp, "active") => Cow::Borrowed("アクティブ"),
            (Self::KoKr, "active") => Cow::Borrowed("활성"),
            (Self::ZhCn, "idle") => Cow::Borrowed("空闲"),
            (Self::ZhTw, "idle") => Cow::Borrowed("閒置"),
            (Self::JaJp, "idle") => Cow::Borrowed("アイドル"),
            (Self::KoKr, "idle") => Cow::Borrowed("유휴"),
            (Self::ZhCn, "not loaded") => Cow::Borrowed("未加载"),
            (Self::ZhTw, "not loaded") => Cow::Borrowed("未載入"),
            (Self::JaJp, "not loaded") => Cow::Borrowed("未読込"),
            (Self::KoKr, "not loaded") => Cow::Borrowed("로드되지 않음"),
            (Self::ZhCn, "error") => Cow::Borrowed("错误"),
            (Self::ZhTw, "error") => Cow::Borrowed("錯誤"),
            (Self::JaJp, "error") => Cow::Borrowed("エラー"),
            (Self::KoKr, "error") => Cow::Borrowed("오류"),
            (_, state) => Cow::Owned(state.to_string()),
        }
    }

    pub(crate) fn resume_empty(self) -> &'static str {
        match self {
            Self::ZhCn => "没有可恢复的会话。按 Esc 取消。",
            Self::ZhTw => "沒有可恢復的工作階段。按 Esc 取消。",
            Self::EnUs => "No resumable conversations. Press Esc to cancel.",
            Self::JaJp => "再開できる会話がありません。Esc でキャンセル。",
            Self::KoKr => "재개할 수 있는 대화가 없습니다. Esc로 취소.",
        }
    }

    pub(crate) fn resume_loading(self) -> &'static str {
        match self {
            Self::ZhCn => "正在加载会话…",
            Self::ZhTw => "正在載入工作階段…",
            Self::EnUs => "Loading conversations…",
            Self::JaJp => "会話を読み込み中…",
            Self::KoKr => "대화 로드 중…",
        }
    }

    pub(crate) fn resume_status_label(self, archived: bool) -> &'static str {
        match (self, archived) {
            (Self::ZhCn, false) => "活动",
            (Self::ZhCn, true) => "已归档",
            (Self::ZhTw, false) => "使用中",
            (Self::ZhTw, true) => "已封存",
            (Self::EnUs, false) => "Active",
            (Self::EnUs, true) => "Archived",
            (Self::JaJp, false) => "アクティブ",
            (Self::JaJp, true) => "アーカイブ済み",
            (Self::KoKr, false) => "활성",
            (Self::KoKr, true) => "보관됨",
        }
    }

    pub(crate) fn resume_filter_label(self, show_all: bool) -> &'static str {
        match (self, show_all) {
            (Self::ZhCn, false) => "当前目录",
            (Self::ZhCn, true) => "全部目录",
            (Self::ZhTw, false) => "目前目錄",
            (Self::ZhTw, true) => "所有目錄",
            (Self::EnUs, false) => "Current cwd",
            (Self::EnUs, true) => "All directories",
            (Self::JaJp, false) => "現在のディレクトリ",
            (Self::JaJp, true) => "すべてのディレクトリ",
            (Self::KoKr, false) => "현재 디렉터리",
            (Self::KoKr, true) => "모든 디렉터리",
        }
    }

    pub(crate) fn resume_sort_label(self, created: bool) -> &'static str {
        match (self, created) {
            (Self::ZhCn, false) => "按更新时间",
            (Self::ZhCn, true) => "按创建时间",
            (Self::ZhTw, false) => "依更新時間",
            (Self::ZhTw, true) => "依建立時間",
            (Self::EnUs, false) => "Updated",
            (Self::EnUs, true) => "Created",
            (Self::JaJp, false) => "更新日時順",
            (Self::JaJp, true) => "作成日時順",
            (Self::KoKr, false) => "업데이트순",
            (Self::KoKr, true) => "생성순",
        }
    }

    pub(crate) fn resume_density_label(self, dense: bool) -> &'static str {
        match (self, dense) {
            (Self::ZhCn, false) => "紧凑视图",
            (Self::ZhCn, true) => "舒适视图",
            (Self::ZhTw, false) => "緊湊檢視",
            (Self::ZhTw, true) => "舒適檢視",
            (Self::EnUs, false) => "Dense view",
            (Self::EnUs, true) => "Comfortable view",
            (Self::JaJp, false) => "コンパクト表示",
            (Self::JaJp, true) => "標準表示",
            (Self::KoKr, false) => "밀집 보기",
            (Self::KoKr, true) => "편안한 보기",
        }
    }

    pub(crate) fn resume_search_placeholder(self) -> &'static str {
        match self {
            Self::ZhCn => "输入以搜索会话",
            Self::ZhTw => "輸入以搜尋工作階段",
            Self::EnUs => "Type to search conversations",
            Self::JaJp => "入力して会話を検索",
            Self::KoKr => "입력하여 대화 검색",
        }
    }

    pub(crate) fn resume_picker_title(self, fork: bool) -> &'static str {
        match (self, fork) {
            (Self::ZhCn, false) => "恢复之前的会话",
            (Self::ZhCn, true) => "从之前的会话分叉",
            (Self::ZhTw, false) => "恢復先前的工作階段",
            (Self::ZhTw, true) => "從先前的工作階段分支",
            (Self::EnUs, false) => "Resume a previous session",
            (Self::EnUs, true) => "Fork a previous session",
            (Self::JaJp, false) => "以前のセッションを再開",
            (Self::JaJp, true) => "以前のセッションを分岐",
            (Self::KoKr, false) => "이전 세션 재개",
            (Self::KoKr, true) => "이전 세션에서 분기",
        }
    }

    pub(crate) fn resume_enter_hint(self, fork: bool, archived: bool) -> &'static str {
        match (self, fork, archived) {
            (Self::ZhCn, _, true) => "Enter 恢复",
            (Self::ZhCn, false, false) => "Enter 恢复",
            (Self::ZhCn, true, false) => "Enter 分叉",
            (Self::ZhTw, _, true) => "Enter 恢復",
            (Self::ZhTw, false, false) => "Enter 恢復",
            (Self::ZhTw, true, false) => "Enter 分支",
            (Self::EnUs, _, true) => "Enter restore",
            (Self::EnUs, false, false) => "Enter resume",
            (Self::EnUs, true, false) => "Enter fork",
            (Self::JaJp, _, true) => "Enter 再開",
            (Self::JaJp, false, false) => "Enter 再開",
            (Self::JaJp, true, false) => "Enter 分岐",
            (Self::KoKr, _, true) => "Enter 복원",
            (Self::KoKr, false, false) => "Enter 재개",
            (Self::KoKr, true, false) => "Enter 분기",
        }
    }

    pub(crate) fn resume_controls_hint(self) -> &'static str {
        match self {
            Self::ZhCn => {
                "Esc 新建  Ctrl+C 退出  Ctrl+S 状态  Ctrl+F 目录  Ctrl+R 排序  Ctrl+O 密度"
            }
            Self::ZhTw => {
                "Esc 新建  Ctrl+C 離開  Ctrl+S 狀態  Ctrl+F 目錄  Ctrl+R 排序  Ctrl+O 密度"
            }
            Self::EnUs => {
                "Esc new  Ctrl+C quit  Ctrl+S status  Ctrl+F directory  Ctrl+R sort  Ctrl+O density"
            }
            Self::JaJp => {
                "Esc 新規  Ctrl+C 終了  Ctrl+S 状態  Ctrl+F ディレクトリ  Ctrl+R 並び順  Ctrl+O 密度"
            }
            Self::KoKr => {
                "Esc 새로 만들기  Ctrl+C 종료  Ctrl+S 상태  Ctrl+F 디렉터리  Ctrl+R 정렬  Ctrl+O 밀도"
            }
        }
    }

    pub(crate) fn resume_expand_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "Ctrl+E 展开记录",
            Self::ZhTw => "Ctrl+E 展開記錄",
            Self::EnUs => "Ctrl+E expand transcript",
            Self::JaJp => "Ctrl+E 履歴を展開",
            Self::KoKr => "Ctrl+E 대화 기록 펼치기",
        }
    }

    pub(crate) fn resume_transcript_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "Ctrl+T 查看完整记录",
            Self::ZhTw => "Ctrl+T 檢視完整記錄",
            Self::EnUs => "Ctrl+T view full transcript",
            Self::JaJp => "Ctrl+T 完全な履歴を表示",
            Self::KoKr => "Ctrl+T 전체 대화 기록 보기",
        }
    }

    pub(crate) fn created_label(self) -> &'static str {
        match self {
            Self::ZhCn => "创建时间",
            Self::ZhTw => "建立時間",
            Self::EnUs => "created",
            Self::JaJp => "作成日時",
            Self::KoKr => "생성 시간",
        }
    }

    pub(crate) fn updated_label(self) -> &'static str {
        match self {
            Self::ZhCn => "更新时间",
            Self::ZhTw => "更新時間",
            Self::EnUs => "updated",
            Self::JaJp => "更新日時",
            Self::KoKr => "업데이트 시간",
        }
    }

    pub(crate) fn resume_transcript_loading(self) -> &'static str {
        match self {
            Self::ZhCn => "正在加载对话记录…",
            Self::ZhTw => "正在載入對話記錄…",
            Self::EnUs => "Loading transcript…",
            Self::JaJp => "会話履歴を読み込み中…",
            Self::KoKr => "대화 기록 로드 중…",
        }
    }

    pub(crate) fn resume_transcript_failed(self) -> &'static str {
        match self {
            Self::ZhCn => "无法加载对话记录",
            Self::ZhTw => "無法載入對話記錄",
            Self::EnUs => "Could not load transcript",
            Self::JaJp => "会話履歴を読み込めません",
            Self::KoKr => "대화 기록을 불러올 수 없음",
        }
    }

    pub(crate) fn resume_transcript_empty(self) -> &'static str {
        match self {
            Self::ZhCn => "没有对话内容",
            Self::ZhTw => "沒有對話內容",
            Self::EnUs => "No transcript content",
            Self::JaJp => "会話内容がありません",
            Self::KoKr => "대화 내용 없음",
        }
    }

    pub(crate) fn no_questions(self) -> &'static str {
        match self {
            Self::ZhCn => "没有问题",
            Self::ZhTw => "沒有問題",
            Self::EnUs => "No questions",
            Self::JaJp => "質問はありません",
            Self::KoKr => "질문 없음",
        }
    }

    pub(crate) fn other_option(self) -> &'static str {
        match self {
            Self::ZhCn => "其他  输入自定义答案",
            Self::ZhTw => "其他  輸入自訂答案",
            Self::EnUs => "Other  Type a custom answer",
            Self::JaJp => "その他  カスタム回答を入力",
            Self::KoKr => "기타  사용자 지정 답변 입력",
        }
    }

    pub(crate) fn add_notes(self) -> &'static str {
        match self {
            Self::ZhCn => "按 Tab 添加备注",
            Self::ZhTw => "按 Tab 新增備註",
            Self::EnUs => "Tab to add notes",
            Self::JaJp => "Tab でメモを追加",
            Self::KoKr => "Tab으로 메모 추가",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Locale;
    use crate::slash_command::SlashCommand;

    #[test]
    fn parses_supported_locale_families_and_falls_back_to_english() {
        assert_eq!(Locale::parse("zh_CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::parse("zh-Hant-TW"), Some(Locale::ZhTw));
        assert_eq!(Locale::parse("ja"), Some(Locale::JaJp));
        assert_eq!(Locale::parse("ko-KR"), Some(Locale::KoKr));
        assert_eq!(Locale::parse("en-GB"), Some(Locale::EnUs));
        assert_eq!(Locale::parse("fr-FR"), None);
    }

    #[test]
    fn translates_statuses_and_dynamic_detail_labels_without_changing_unknown_text() {
        assert_eq!(Locale::ZhCn.status("ready"), "就绪");
        assert_eq!(Locale::JaJp.status("effort: high"), "推論: high");
        assert_eq!(
            Locale::ZhTw.status("image attached: 20x10"),
            "已附加圖片: 20x10"
        );
        assert_eq!(
            Locale::KoKr.status("image paste failed: unavailable"),
            "이미지 붙여넣기 실패: unavailable"
        );
        assert_eq!(Locale::KoKr.detail("files: 2"), "파일 수: 2");
        assert_eq!(Locale::ZhCn.detail("model: fixture"), "模型：fixture");
        assert_eq!(Locale::ZhTw.detail("effort: high"), "思考強度：high");
        assert_eq!(Locale::EnUs.detail("prompt: inspect"), "prompt: inspect");
        assert_eq!(Locale::JaJp.detail("model: fixture"), "モデル: fixture");
        assert_eq!(Locale::KoKr.detail("effort: high"), "추론 강도: high");
        assert_eq!(Locale::ZhCn.detail("output: ok"), "输出：ok");
        assert_eq!(Locale::ZhTw.detail("output: ok"), "輸出：ok");
        assert_eq!(Locale::EnUs.detail("output: ok"), "output: ok");
        assert_eq!(Locale::JaJp.detail("output: ok"), "出力: ok");
        assert_eq!(Locale::KoKr.detail("output: ok"), "출력: ok");
        assert_eq!(
            Locale::ZhCn.detail("structured content: {\"matches\":1}"),
            "结构化内容：{\"matches\":1}"
        );
        assert_eq!(
            Locale::ZhTw.detail("structured content: {\"matches\":1}"),
            "結構化內容：{\"matches\":1}"
        );
        assert_eq!(
            Locale::EnUs.detail("structured content: {\"matches\":1}"),
            "structured content: {\"matches\":1}"
        );
        assert_eq!(
            Locale::JaJp.detail("structured content: {\"matches\":1}"),
            "構造化コンテンツ: {\"matches\":1}"
        );
        assert_eq!(
            Locale::KoKr.detail("structured content: {\"matches\":1}"),
            "구조화 콘텐츠: {\"matches\":1}"
        );
        assert_eq!(
            Locale::ZhCn.detail("computer screenshot: captured"),
            "电脑截图：captured"
        );
        assert_eq!(
            Locale::ZhTw.detail("computer action: Capture"),
            "電腦操作：Capture"
        );
        assert_eq!(
            Locale::EnUs.detail("computer error: failed"),
            "computer error: failed"
        );
        assert_eq!(
            Locale::JaJp.detail("computer screenshot: captured"),
            "コンピュータ スクリーンショット: captured"
        );
        assert_eq!(
            Locale::KoKr.detail("computer action: Capture"),
            "컴퓨터 작업: Capture"
        );
        assert_eq!(Locale::ZhTw.detail("custom: value"), "custom: value");
        assert_eq!(Locale::ZhCn.status("running hook"), "正在运行 Hook");
        assert_eq!(Locale::ZhTw.status("running hooks"), "正在執行多個 Hook");
        assert_eq!(Locale::EnUs.status("running hook"), "running hook");
        assert_eq!(Locale::JaJp.status("running hooks"), "複数の Hook を実行中");
        assert_eq!(Locale::KoKr.status("running hook"), "Hook 실행 중");
        assert_eq!(
            Locale::ZhCn.status("MCP startup issue: docs: offline"),
            "MCP 启动问题: docs: offline"
        );
        assert_eq!(
            Locale::ZhTw.status("MCP startup issues: 2"),
            "MCP 啟動問題: 2"
        );
        assert_eq!(
            Locale::JaJp.status("MCP startup issue: docs: offline"),
            "MCP 起動の問題: docs: offline"
        );
        assert_eq!(
            Locale::KoKr.status("MCP startup issue: docs: offline"),
            "MCP 시작 문제: docs: offline"
        );
        assert_eq!(Locale::ZhCn.detail("hook completed"), "Hook 已完成");
        assert_eq!(Locale::ZhTw.detail("hook failed"), "Hook 失敗");
        assert_eq!(Locale::EnUs.detail("hook blocked"), "hook blocked");
        assert_eq!(Locale::JaJp.detail("hook stopped"), "Hook 停止");
        assert_eq!(Locale::KoKr.detail("hook output: ok"), "Hook 출력: ok");
        assert_eq!(Locale::ZhCn.approval_option("Allow once"), "允许一次");
        assert_eq!(
            Locale::JaJp.approval_option("Cancel turn"),
            "ターンをキャンセル"
        );
    }

    #[test]
    fn collaboration_mode_statuses_cover_all_product_locales() {
        let cases = [
            (
                Locale::ZhCn,
                "协作模式已更新",
                "计划模式",
                "此服务器不支持计划模式",
            ),
            (
                Locale::ZhTw,
                "協作模式已更新",
                "計畫模式",
                "此伺服器不支援計畫模式",
            ),
            (
                Locale::EnUs,
                "collaboration mode updated",
                "plan mode",
                "plan mode unavailable on this server",
            ),
            (
                Locale::JaJp,
                "コラボレーションモードを更新しました",
                "計画モード",
                "このサーバーでは計画モードを利用できません",
            ),
            (
                Locale::KoKr,
                "협업 모드가 업데이트됨",
                "계획 모드",
                "이 서버에서는 계획 모드를 사용할 수 없음",
            ),
        ];

        for (locale, updated, plan, unavailable) in cases {
            assert_eq!(locale.status("collaboration mode updated"), updated);
            assert_eq!(locale.status("plan mode"), plan);
            assert_eq!(
                locale.status("plan mode unavailable on this server"),
                unavailable
            );
        }
    }

    #[test]
    fn multi_agent_titles_cover_all_product_locales() {
        let cases = [
            (
                Locale::ZhCn,
                "已启动 agent-1",
                "等待完成",
                "已中断 `agent-1`",
            ),
            (
                Locale::ZhTw,
                "已啟動 agent-1",
                "等待完成",
                "已中斷 `agent-1`",
            ),
            (
                Locale::EnUs,
                "Spawned agent-1",
                "Finished waiting",
                "Interrupted `agent-1`",
            ),
            (
                Locale::JaJp,
                "起動: agent-1",
                "待機が完了しました",
                "中断: `agent-1`",
            ),
            (
                Locale::KoKr,
                "생성됨 agent-1",
                "대기 완료",
                "중단됨 `agent-1`",
            ),
        ];
        for (locale, spawned, waiting, interrupted) in cases {
            assert_eq!(locale.multi_agent("Spawned agent-1"), spawned);
            assert_eq!(locale.multi_agent("Finished waiting"), waiting);
            assert_eq!(locale.multi_agent("Interrupted `agent-1`"), interrupted);
        }
        assert_eq!(Locale::EnUs.multi_agent("unknown event"), "unknown event");
    }

    #[test]
    fn command_output_omission_markers_cover_all_product_locales() {
        let line_markers = [
            (Locale::ZhCn, "已省略 2 行"),
            (Locale::ZhTw, "已省略 2 行"),
            (Locale::EnUs, "2 lines omitted"),
            (Locale::JaJp, "2 行を省略"),
            (Locale::KoKr, "2줄 생략"),
        ];
        let byte_markers = [
            (Locale::ZhCn, "已省略 3 字节"),
            (Locale::ZhTw, "已省略 3 位元組"),
            (Locale::EnUs, "3 bytes omitted"),
            (Locale::JaJp, "3 バイトを省略"),
            (Locale::KoKr, "3바이트 생략"),
        ];

        for (locale, marker) in line_markers {
            assert!(locale.output_omitted_lines(2).contains(marker));
        }
        for (locale, marker) in byte_markers {
            assert!(locale.output_omitted_bytes(3).contains(marker));
        }
    }

    #[test]
    fn agents_overview_controls_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            assert!(!locale.agents_overview_title().is_empty());
            assert!(!locale.agents_overview_footer().is_empty());
            assert!(!locale.agents_overview_search_prefix().is_empty());
            assert!(!locale.agents_overview_group_label("need input").is_empty());
        }
    }

    #[test]
    fn agents_overview_task_statuses_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            assert!(!locale.agents_overview_input_prefix(false).is_empty());
            assert!(!locale.agents_overview_input_prefix(true).is_empty());
            assert!(!locale
                .status("background task started: thread-1")
                .is_empty());
            assert!(!locale.status("background task failed: offline").is_empty());
            assert!(!locale.status("agent renamed").is_empty());
            assert!(!locale
                .status("resume picker is available from /resume")
                .is_empty());
        }
    }

    #[test]
    fn image_clipboard_statuses_cover_all_product_locales() {
        let cases = [
            (
                Locale::ZhCn,
                "已附加图片: 20x10",
                "粘贴图片失败: unavailable",
            ),
            (
                Locale::ZhTw,
                "已附加圖片: 20x10",
                "貼上圖片失敗: unavailable",
            ),
            (
                Locale::EnUs,
                "image attached: 20x10",
                "image paste failed: unavailable",
            ),
            (
                Locale::JaJp,
                "画像を添付しました: 20x10",
                "画像の貼り付けに失敗しました: unavailable",
            ),
            (
                Locale::KoKr,
                "이미지 첨부됨: 20x10",
                "이미지 붙여넣기 실패: unavailable",
            ),
        ];

        for (locale, attached, failed) in cases {
            assert_eq!(locale.status("image attached: 20x10"), attached);
            assert_eq!(locale.status("image paste failed: unavailable"), failed);
        }
    }

    #[test]
    fn queue_unavailable_status_covers_all_product_locales() {
        let cases = [
            (
                Locale::ZhCn,
                "队列不可用: offline",
                "正在编辑排队输入",
                "排队输入已不可用",
                "编辑排队输入失败: offline",
                "Alt+Up 编辑最后一条排队输入",
            ),
            (
                Locale::ZhTw,
                "佇列無法使用: offline",
                "正在編輯排隊輸入",
                "排隊輸入已無法使用",
                "編輯排隊輸入失敗: offline",
                "Alt+Up 編輯最後一則排隊輸入",
            ),
            (
                Locale::EnUs,
                "queue unavailable: offline",
                "editing queued input",
                "queued input unavailable",
                "queue edit failed: offline",
                "Alt+Up edit last queued input",
            ),
            (
                Locale::JaJp,
                "キューを利用できません: offline",
                "キュー入力を編集中",
                "キュー入力を利用できません",
                "キュー入力の編集に失敗しました: offline",
                "Alt+Up 最後のキュー入力を編集",
            ),
            (
                Locale::KoKr,
                "대기열을 사용할 수 없음: offline",
                "대기 입력 편집 중",
                "대기 입력을 사용할 수 없음",
                "대기 입력 편집 실패: offline",
                "Alt+Up 마지막 대기 입력 편집",
            ),
        ];
        for (locale, unavailable, editing, input_unavailable, failed, hint) in cases {
            assert_eq!(locale.status("queue unavailable: offline"), unavailable);
            assert_eq!(locale.status("queued input editing"), editing);
            assert_eq!(locale.status("queued input unavailable"), input_unavailable);
            assert_eq!(locale.status("queue edit failed: offline"), failed);
            assert_eq!(locale.edit_queued_input_hint(), hint);
        }
    }

    #[test]
    fn exposes_all_product_locale_tags() {
        let locales = [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ];
        assert_eq!(
            locales
                .iter()
                .map(|locale| locale.tag())
                .collect::<Vec<_>>(),
            vec!["zh-CN", "zh-TW", "en-US", "ja-JP", "ko-KR"]
        );
    }

    #[test]
    fn slash_command_descriptions_cover_all_product_locales() {
        let cases = [
            (Locale::ZhCn, "选择模型"),
            (Locale::ZhTw, "選擇模型"),
            (Locale::EnUs, "choose a model"),
            (Locale::JaJp, "モデルを選択"),
            (Locale::KoKr, "모델 선택"),
        ];
        for (locale, expected) in cases {
            assert_eq!(
                locale.slash_command_description(SlashCommand::Model),
                expected
            );
            for command in SlashCommand::ALL {
                assert!(!locale.slash_command_description(command).is_empty());
            }
        }
    }

    #[test]
    fn export_picker_labels_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            for label in [
                locale.export_title(),
                locale.export_subtitle(),
                locale.export_copy_label(),
                locale.export_copy_description(),
                locale.export_file_label(),
                locale.export_file_description(),
                locale.export_picker_hint(),
                locale.export_prompt_title(),
                locale.export_prompt_hint(),
            ] {
                assert!(!label.is_empty(), "missing export label for {locale:?}");
            }
        }
    }

    #[test]
    fn mcp_approval_labels_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            for value in [
                "accept",
                "accept_session",
                "accept_always",
                "decline",
                "cancel",
            ] {
                assert!(
                    !locale.mcp_elicitation_approval_option(value).is_empty(),
                    "missing MCP approval label for {locale:?}/{value}"
                );
            }
        }
    }

    #[test]
    fn working_directory_messages_cover_all_product_locales() {
        let cwd = "/tmp/project";
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let message = locale.current_working_directory_message(cwd);
            assert!(message.contains(cwd));
            assert!(!locale.pwd_usage().is_empty());
        }
    }

    #[test]
    fn file_search_popup_labels_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            assert!(!locale.file_search_loading().is_empty());
            assert!(!locale.file_search_no_matches().is_empty());
        }
        assert_eq!(Locale::ZhCn.file_search_loading(), "正在搜索...");
        assert_eq!(Locale::EnUs.file_search_no_matches(), "no matches");
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            assert!(!locale.skill_popup_no_matches().is_empty());
        }
    }

    #[test]
    fn skill_warning_messages_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let summary = locale.skipped_skills_message(2);
            let detail = locale.skill_load_error_message("/tmp/SKILL.md", "invalid");
            assert!(summary.contains('2'));
            assert!(detail.contains("/tmp/SKILL.md"));
            assert!(detail.contains("invalid"));
        }
    }

    #[test]
    fn status_pager_labels_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            for label in [
                locale.status_title(),
                locale.transcript_title(),
                locale.thread_label(),
                locale.provider_label(),
                locale.cwd_label(),
                locale.state_label(),
                locale.not_set_label(),
                locale.history_search_label(),
                locale.pager_footer(),
                locale.transcript_pager_footer(),
            ] {
                assert!(!label.is_empty(), "{locale:?}");
            }
        }
        assert_eq!(Locale::ZhCn.status_title(), "状态");
        assert_eq!(Locale::EnUs.transcript_title(), "T R A N S C R I P T");
        assert_eq!(Locale::ZhTw.state_label(), "狀態");
        assert_eq!(Locale::JaJp.not_set_label(), "未設定");
        assert!(Locale::KoKr.pager_footer().contains("Esc/Q"));
        assert!(Locale::ZhCn.transcript_pager_footer().contains("Ctrl+T"));
    }

    #[test]
    fn resume_picker_controls_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            assert!(!locale.resume_picker_title(false).is_empty());
            assert!(!locale.resume_picker_title(true).is_empty());
            assert!(!locale.resume_enter_hint(false, false).is_empty());
            assert!(!locale.resume_enter_hint(true, false).is_empty());
            assert!(!locale.resume_enter_hint(false, true).is_empty());
            assert!(!locale.resume_controls_hint().is_empty());
            assert!(!locale.resume_search_placeholder().is_empty());
            assert!(!locale.resume_loading().is_empty());
            assert!(!locale.resume_status_label(false).is_empty());
            assert!(!locale.resume_status_label(true).is_empty());
            assert!(!locale.resume_filter_label(false).is_empty());
            assert!(!locale.resume_filter_label(true).is_empty());
            assert!(!locale.resume_sort_label(false).is_empty());
            assert!(!locale.resume_sort_label(true).is_empty());
            assert!(!locale.resume_density_label(false).is_empty());
            assert!(!locale.resume_density_label(true).is_empty());
            assert!(!locale.resume_expand_hint().is_empty());
            assert!(!locale.resume_transcript_hint().is_empty());
            assert!(!locale.created_label().is_empty());
            assert!(!locale.updated_label().is_empty());
            assert!(!locale.resume_transcript_loading().is_empty());
            assert!(!locale.resume_transcript_failed().is_empty());
            assert!(!locale.resume_transcript_empty().is_empty());
        }
    }
}

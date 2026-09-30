//! Localized shortcut reference labels; keys always come from their routing owner.

use super::Locale;

#[derive(Clone, Copy)]
pub(crate) enum ShortcutLabel {
    Title,
    Compose,
    Session,
    Transcript,
    Commands,
    MentionFiles,
    Skills,
    NewLine,
    PasteImage,
    ExternalEditor,
    SearchHistory,
    QueueMessage,
    SendMessage,
    ChangeMode,
    LessReasoning,
    MoreReasoning,
    Agents,
    InspectActivity,
    Interrupt,
    Quit,
    OpenTranscript,
    FindText,
    ScrollUp,
    ScrollDown,
    Top,
    Latest,
    StartSelection,
    Close,
    Customize,
    Resize,
}

impl Locale {
    pub(crate) fn shortcut_label(self, label: ShortcutLabel) -> &'static str {
        use ShortcutLabel::*;
        let labels = match label {
            Title => [
                "键盘快捷键",
                "鍵盤快捷鍵",
                "Keyboard shortcuts",
                "キーボードショートカット",
                "키보드 단축키",
            ],
            Compose => ["编辑", "編輯", "Compose", "入力", "작성"],
            Session => ["会话", "工作階段", "Session", "セッション", "세션"],
            Transcript => [
                "记录（先打开）",
                "記錄（先開啟）",
                "Transcript (open first)",
                "会話履歴（先に開く）",
                "대화 기록 (먼저 열기)",
            ],
            Commands => ["命令", "命令", "Commands", "コマンド", "명령"],
            MentionFiles => [
                "引用文件",
                "引用檔案",
                "Mention files",
                "ファイルを参照",
                "파일 참조",
            ],
            Skills => ["技能", "技能", "Skills", "スキル", "스킬"],
            NewLine => ["换行", "換行", "New line", "改行", "줄 바꿈"],
            PasteImage => [
                "粘贴图片",
                "貼上圖片",
                "Paste image",
                "画像を貼り付け",
                "이미지 붙여넣기",
            ],
            ExternalEditor => [
                "外部编辑器",
                "外部編輯器",
                "External editor",
                "外部エディター",
                "외부 편집기",
            ],
            SearchHistory => [
                "搜索历史",
                "搜尋歷史",
                "Search history",
                "履歴を検索",
                "입력 기록 검색",
            ],
            QueueMessage => [
                "排队消息",
                "排隊訊息",
                "Queue message",
                "メッセージをキューに追加",
                "메시지 대기열 추가",
            ],
            SendMessage => [
                "发送消息",
                "傳送訊息",
                "Send message",
                "メッセージを送信",
                "메시지 보내기",
            ],
            ChangeMode => [
                "切换模式",
                "切換模式",
                "Change mode",
                "モードを切り替え",
                "모드 전환",
            ],
            LessReasoning => [
                "降低推理",
                "降低推理",
                "Less reasoning",
                "推論を減らす",
                "추론 줄이기",
            ],
            MoreReasoning => [
                "增强推理",
                "增強推理",
                "More reasoning",
                "推論を増やす",
                "추론 늘리기",
            ],
            Agents => [
                "任务（空输入）",
                "任務（空輸入）",
                "Agents (empty prompt)",
                "エージェント（空の入力）",
                "에이전트 (빈 입력)",
            ],
            InspectActivity => [
                "查看活动",
                "檢視活動",
                "Inspect activity",
                "アクティビティを確認",
                "활동 확인",
            ],
            Interrupt => ["中断", "中斷", "Interrupt", "中断", "중단"],
            Quit => ["退出", "結束", "Quit", "終了", "종료"],
            OpenTranscript => [
                "打开记录",
                "開啟記錄",
                "Open transcript",
                "会話履歴を開く",
                "대화 기록 열기",
            ],
            FindText => [
                "查找文本",
                "尋找文字",
                "Find text",
                "テキストを検索",
                "텍스트 찾기",
            ],
            ScrollUp => [
                "向上滚动",
                "向上捲動",
                "Scroll up",
                "上にスクロール",
                "위로 스크롤",
            ],
            ScrollDown => [
                "向下滚动",
                "向下捲動",
                "Scroll down",
                "下にスクロール",
                "아래로 스크롤",
            ],
            Top => ["顶部", "頂部", "Top", "先頭", "맨 위"],
            Latest => ["最新", "最新", "Latest", "最新", "최신"],
            StartSelection => [
                "开始选择",
                "開始選取",
                "Start selection",
                "選択を開始",
                "선택 시작",
            ],
            Close => ["关闭", "關閉", "close", "閉じる", "닫기"],
            Customize => [
                "自定义快捷键",
                "自訂快捷鍵",
                "customize bindings",
                "キー設定を変更",
                "단축키 사용자 지정",
            ],
            Resize => [
                "… 调整窗口查看全部",
                "… 調整視窗查看全部",
                "… resize to see all",
                "… サイズを変更して全体を表示",
                "… 크기를 조정해 모두 보기",
            ],
        };
        labels[match self {
            Self::ZhCn => 0,
            Self::ZhTw => 1,
            Self::EnUs => 2,
            Self::JaJp => 3,
            Self::KoKr => 4,
        }]
    }

    pub(crate) fn agents_key_hint(self, key: &str) -> String {
        match self {
            Self::ZhCn => format!("{key} 查看任务"),
            Self::ZhTw => format!("{key} 檢視任務"),
            Self::EnUs => format!("{key} for agents"),
            Self::JaJp => format!("{key} エージェント"),
            Self::KoKr => format!("{key} 에이전트"),
        }
    }
}

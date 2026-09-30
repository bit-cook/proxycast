//! Model picker labels share the product locales; catalog facts remain server-owned.

use super::Locale;

impl Locale {
    pub(crate) fn approval_header_elision(self, lines: usize) -> String {
        match self {
            Self::ZhCn => format!("[… {lines} 行] ctrl+a 查看全文"),
            Self::ZhTw => format!("[… {lines} 行] ctrl+a 查看全文"),
            Self::EnUs => format!("[… {lines} lines] ctrl+a view all"),
            Self::JaJp => format!("[… {lines} 行] ctrl+a 全文を表示"),
            Self::KoKr => format!("[… {lines}줄] ctrl+a 전체 보기"),
        }
    }

    pub(crate) fn completion_skill_tag(self) -> &'static str {
        match self {
            Self::ZhCn => "[技能]",
            Self::ZhTw => "[技能]",
            Self::EnUs => "[Skill]",
            Self::JaJp => "[スキル]",
            Self::KoKr => "[스킬]",
        }
    }

    pub(crate) fn skill_popup_footer(self) -> &'static str {
        match self {
            Self::ZhCn => "enter 插入 · esc 关闭",
            Self::ZhTw => "enter 插入 · esc 關閉",
            Self::EnUs => "enter insert · esc close",
            Self::JaJp => "enter 挿入 · esc 閉じる",
            Self::KoKr => "enter 삽입 · esc 닫기",
        }
    }

    pub(crate) fn model_picker_search_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "输入以筛选模型",
            Self::ZhTw => "輸入以篩選模型",
            Self::EnUs => "Type to filter models",
            Self::JaJp => "入力してモデルを絞り込む",
            Self::KoKr => "입력하여 모델 필터링",
        }
    }
    pub(crate) fn model_picker_title(self) -> &'static str {
        match self {
            Self::ZhCn => "选择模型",
            Self::ZhTw => "選擇模型",
            Self::EnUs => "Select model",
            Self::JaJp => "モデルを選択",
            Self::KoKr => "모델 선택",
        }
    }

    pub(crate) fn picker_current_label(self) -> &'static str {
        match self {
            Self::ZhCn => "当前",
            Self::ZhTw => "目前",
            Self::EnUs => "current",
            Self::JaJp => "現在",
            Self::KoKr => "현재",
        }
    }

    pub(crate) fn picker_default_label(self) -> &'static str {
        match self {
            Self::ZhCn => "默认",
            Self::ZhTw => "預設",
            Self::EnUs => "default",
            Self::JaJp => "デフォルト",
            Self::KoKr => "기본값",
        }
    }

    pub(crate) fn model_picker_footer(self) -> &'static str {
        match self {
            Self::ZhCn => "Enter 选择 · Esc 返回",
            Self::ZhTw => "Enter 選擇 · Esc 返回",
            Self::EnUs => "enter select · esc back",
            Self::JaJp => "Enter 選択 · Esc 戻る",
            Self::KoKr => "Enter 선택 · Esc 돌아가기",
        }
    }
}

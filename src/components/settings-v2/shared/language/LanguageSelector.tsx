/**
 * Language Selector Component
 *
 * Dropdown component for selecting the UI language.
 * Similar design to the theme selector in GeneralSettings.
 *
 * Features:
 * - Displays available languages (Chinese, English)
 * - Highlights currently selected language
 * - Persists selection to config
 * - Updates UI immediately via Patch Layer
 */

import { cn } from "@/lib/utils";

export type Language = "zh" | "en";

interface LanguageOption {
  id: Language;
  label: string;
  nativeName: string;
}

interface LanguageSelectorProps {
  currentLanguage: Language;
  onLanguageChange: (language: Language) => void;
  disabled?: boolean;
}

const languageOptions: LanguageOption[] = [
  { id: "zh", label: "中文", nativeName: "Chinese" },
  { id: "en", label: "English", nativeName: "English" },
];

/**
 * Language Selector Component
 *
 * A simple button-based language selector similar to the theme selector.
 * Each button shows both the native name and English label.
 */
export function LanguageSelector({
  currentLanguage,
  onLanguageChange,
  disabled = false,
}: LanguageSelectorProps) {
  return (
    <div className="flex gap-1">
      {languageOptions.map((option) => (
        <button
          key={option.id}
          onClick={() => onLanguageChange(option.id)}
          disabled={disabled}
          className={cn(
            "flex items-center gap-1.5 px-3 py-1 rounded text-sm transition-colors",
            currentLanguage === option.id
              ? "bg-primary/10 text-primary"
              : "hover:bg-muted",
            disabled && "opacity-50 cursor-not-allowed",
          )}
          title={option.nativeName}
        >
          <span className="font-medium">{option.label}</span>
          <span className="text-xs text-muted-foreground">
            ({option.nativeName})
          </span>
        </button>
      ))}
    </div>
  );
}

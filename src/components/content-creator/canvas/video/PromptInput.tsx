import React, { memo, KeyboardEvent, useRef } from "react";
import styled from "styled-components";
import { VideoCanvasState } from "./types";
import { Sparkles } from "lucide-react";
import { CharacterMention } from "@/components/agent/chat/components/Inputbar/components/CharacterMention";
import { SkillBadge } from "@/components/agent/chat/components/Inputbar/components/SkillBadge";
import { useActiveSkill } from "@/components/agent/chat/components/Inputbar/hooks/useActiveSkill";
import type { Skill } from "@/lib/api/skills";

interface PromptInputProps {
  state: VideoCanvasState;
  onStateChange: (state: VideoCanvasState) => void;
  onGenerate: (textOverride?: string) => void;
  skills?: Skill[];
}

const PromptWrapper = styled.div`
  width: 100%;
  max-width: 920px;
  margin: 0 auto;
`;

const InputContainer = styled.div`
  display: flex;
  align-items: center;
  background: hsl(var(--background));
  border: 1px solid hsl(var(--border));
  border-radius: 14px;
  padding: 10px 10px 10px 14px;
  min-height: 82px;
  transition: all 0.2s;

  &:focus-within {
    border-color: hsl(var(--border));
  }
`;

const StyledTextarea = styled.textarea`
  flex: 1;
  border: none;
  background: transparent;
  padding: 4px 0;
  min-height: 52px;
  max-height: 160px;
  resize: none;
  font-size: 15px;
  line-height: 1.6;
  color: hsl(var(--foreground));
  outline: none;

  &::placeholder {
    color: hsl(var(--muted-foreground));
  }
`;

const GenerateButton = styled.button<{ $generating?: boolean }>`
  flex-shrink: 0;
  width: 58px;
  height: 58px;
  margin-left: 10px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: ${(props) =>
    props.$generating ? "hsl(var(--muted))" : "hsl(var(--muted) / 0.35)"};
  color: ${(props) =>
    props.$generating ? "hsl(var(--muted-foreground))" : "hsl(var(--muted-foreground))"};
  border: 1px solid hsl(var(--border));
  cursor: ${(props) => (props.$generating ? "not-allowed" : "pointer")};
  transition: all 0.2s;

  &:hover:not(:disabled) {
    background: hsl(var(--muted) / 0.52);
  }

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
`;

export const PromptInput: React.FC<PromptInputProps> = memo(
  ({ state, onStateChange, onGenerate, skills = [] }) => {
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const { activeSkill, setActiveSkill, wrapTextWithSkill, clearActiveSkill } =
      useActiveSkill();

    const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        if (state.prompt.trim() && state.status !== "generating") {
          handleGenerate();
        }
      }
    };

    const handleGenerate = () => {
      const text = activeSkill
        ? wrapTextWithSkill(state.prompt)
        : undefined;
      onGenerate(text);
      clearActiveSkill();
    };

    return (
      <PromptWrapper>
        <InputContainer>
          {/* CharacterMention */}
          {skills.length > 0 && (
            <CharacterMention
              characters={[]}
              skills={skills}
              inputRef={textareaRef}
              value={state.prompt}
              onChange={(val) => onStateChange({ ...state, prompt: val })}
              onSelectSkill={setActiveSkill}
            />
          )}
          {/* Skill Badge */}
          {activeSkill && (
            <SkillBadge skill={activeSkill} onClear={clearActiveSkill} />
          )}
          <StyledTextarea
            ref={textareaRef}
            value={state.prompt}
            onChange={(e) => {
              onStateChange({ ...state, prompt: e.target.value });
              // Auto resize
              e.target.style.height = "auto";
              e.target.style.height = `${Math.min(e.target.scrollHeight, 200)}px`;
            }}
            onKeyDown={handleKeyDown}
            placeholder="描述你想生成的视频内容"
            rows={1}
          />
          <GenerateButton
            disabled={!state.prompt.trim() || state.status === "generating"}
            $generating={state.status === "generating"}
            onClick={handleGenerate}
          >
            <Sparkles size={20} />
          </GenerateButton>
        </InputContainer>
      </PromptWrapper>
    );
  },
);

PromptInput.displayName = "PromptInput";

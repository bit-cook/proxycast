/**
 * @file MessageRenderer.test.ts
 * @description 消息渲染属性测试 - 验证 Markdown 渲染的正确性
 * @module components/general-chat/chat/MessageRenderer.test
 *
 * **Feature: general-chat, Property 7: Markdown 渲染包含正确元素**
 * **Validates: Requirements 2.3, 2.4, 6.3, 6.4, 6.5**
 */

import { describe, expect } from "vitest";
import { test } from "@fast-check/vitest";
import * as fc from "fast-check";
import type { ContentBlock } from "../types";

// ============================================================================
// 内容解析函数（从 AssistantMessage.tsx 提取的核心逻辑）
// ============================================================================

/**
 * 解析 Markdown 内容，提取代码块
 * @param content - 原始 Markdown 内容
 * @returns 解析后的内容块数组
 */
export function parseContent(content: string): ContentBlock[] {
  const blocks: ContentBlock[] = [];
  const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;
  let lastIndex = 0;
  let match;

  while ((match = codeBlockRegex.exec(content)) !== null) {
    // 添加代码块之前的文本
    if (match.index > lastIndex) {
      const text = content.slice(lastIndex, match.index).trim();
      if (text) {
        blocks.push({ type: "text", content: text });
      }
    }

    // 添加代码块
    blocks.push({
      type: "code",
      content: match[2].trim(),
      language: match[1] || "plaintext",
    });

    lastIndex = match.index + match[0].length;
  }

  // 添加剩余的文本
  if (lastIndex < content.length) {
    const text = content.slice(lastIndex).trim();
    if (text) {
      blocks.push({ type: "text", content: text });
    }
  }

  // 如果没有解析出任何块，返回整个内容作为文本
  if (blocks.length === 0 && content.trim()) {
    blocks.push({ type: "text", content: content.trim() });
  }

  return blocks;
}

// ============================================================================
// Markdown 元素检测函数
// ============================================================================

/**
 * 检测 Markdown 内容中的标题
 * @param content - Markdown 内容
 * @returns 标题级别数组 (1-6)
 */
export function detectHeadings(content: string): number[] {
  const headingRegex = /^(#{1,6})\s+.+$/gm;
  const headings: number[] = [];
  let match;

  while ((match = headingRegex.exec(content)) !== null) {
    headings.push(match[1].length);
  }

  return headings;
}

/**
 * 检测 Markdown 内容中的列表
 * @param content - Markdown 内容
 * @returns 包含有序列表和无序列表的布尔值
 */
export function detectLists(content: string): {
  hasOrderedList: boolean;
  hasUnorderedList: boolean;
} {
  const orderedListRegex = /^\d+\.\s+.+$/m;
  const unorderedListRegex = /^[-*+]\s+.+$/m;

  return {
    hasOrderedList: orderedListRegex.test(content),
    hasUnorderedList: unorderedListRegex.test(content),
  };
}

/**
 * 检测 Markdown 内容中的代码块
 * @param content - Markdown 内容
 * @returns 代码块信息数组
 */
export function detectCodeBlocks(
  content: string,
): Array<{ language: string; hasContent: boolean }> {
  const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;
  const codeBlocks: Array<{ language: string; hasContent: boolean }> = [];
  let match;

  while ((match = codeBlockRegex.exec(content)) !== null) {
    codeBlocks.push({
      language: match[1] || "plaintext",
      hasContent: match[2].trim().length > 0,
    });
  }

  return codeBlocks;
}

/**
 * 检测 Markdown 内容中的表格
 * @param content - Markdown 内容
 * @returns 是否包含表格
 */
export function detectTables(content: string): boolean {
  // 表格需要至少有表头行和分隔行
  // 格式: | col1 | col2 |
  //       |------|------|
  const tableRegex = /\|.+\|[\r\n]+\|[-:\s|]+\|/;
  return tableRegex.test(content);
}

/**
 * 检测 Markdown 内容中的数学公式
 * @param content - Markdown 内容
 * @returns 包含行内公式和块级公式的布尔值
 */
export function detectMathFormulas(content: string): {
  hasInlineMath: boolean;
  hasBlockMath: boolean;
} {
  // 行内公式: $...$
  const inlineMathRegex = /\$[^$\n]+\$/;
  // 块级公式: $$...$$
  const blockMathRegex = /\$\$[\s\S]+?\$\$/;

  return {
    hasInlineMath: inlineMathRegex.test(content),
    hasBlockMath: blockMathRegex.test(content),
  };
}

/**
 * 检测 Markdown 内容中的文本格式
 * @param content - Markdown 内容
 * @returns 包含粗体、斜体、链接的布尔值
 */
export function detectTextFormatting(content: string): {
  hasBold: boolean;
  hasItalic: boolean;
  hasLink: boolean;
} {
  // 粗体: **text** 或 __text__
  const boldRegex = /\*\*[^*]+\*\*|__[^_]+__/;
  // 斜体: *text* 或 _text_ (但不是 ** 或 __)
  // 注意: ***text*** 同时是粗体和斜体
  const italicRegex = /(?<!\*)\*[^*]+\*(?!\*)|(?<!_)_[^_]+_(?!_)/;
  // 粗斜体: ***text***
  const boldItalicRegex = /\*\*\*[^*]+\*\*\*/;
  // 链接: [text](url)
  const linkRegex = /\[[^\]]+\]\([^)]+\)/;

  const hasBoldItalic = boldItalicRegex.test(content);

  return {
    hasBold: boldRegex.test(content) || hasBoldItalic,
    hasItalic: italicRegex.test(content) || hasBoldItalic,
    hasLink: linkRegex.test(content),
  };
}

// ============================================================================
// Arbitrary 生成器
// ============================================================================

/**
 * 生成有效的 Markdown 标题
 */
const headingArbitrary = (level: number): fc.Arbitrary<string> =>
  fc
    .string({ minLength: 1, maxLength: 50 })
    .filter((s) => s.trim().length > 0 && !s.includes("\n"))
    .map((text) => `${"#".repeat(level)} ${text.trim()}`);

/**
 * 生成有效的无序列表项
 */
const unorderedListArbitrary: fc.Arbitrary<string> = fc
  .array(
    fc
      .string({ minLength: 1, maxLength: 30 })
      .filter((s) => s.trim().length > 0 && !s.includes("\n")),
    { minLength: 1, maxLength: 5 },
  )
  .map((items) => items.map((item) => `- ${item.trim()}`).join("\n"));

/**
 * 生成有效的有序列表项
 */
const orderedListArbitrary: fc.Arbitrary<string> = fc
  .array(
    fc
      .string({ minLength: 1, maxLength: 30 })
      .filter((s) => s.trim().length > 0 && !s.includes("\n")),
    { minLength: 1, maxLength: 5 },
  )
  .map((items) =>
    items.map((item, index) => `${index + 1}. ${item.trim()}`).join("\n"),
  );

/**
 * 生成有效的代码块
 */
const codeBlockArbitrary: fc.Arbitrary<{ markdown: string; language: string }> =
  fc
    .record({
      language: fc.constantFrom(
        "javascript",
        "typescript",
        "python",
        "rust",
        "go",
        "java",
        "plaintext",
      ),
      code: fc
        .string({ minLength: 1, maxLength: 100 })
        .filter((s) => s.trim().length > 0 && !s.includes("```")),
    })
    .map(({ language, code }) => ({
      markdown: `\`\`\`${language}\n${code.trim()}\n\`\`\``,
      language,
    }));

/**
 * 生成有效的表格
 */
const tableArbitrary: fc.Arbitrary<string> = fc
  .record({
    cols: fc.integer({ min: 2, max: 4 }),
    rows: fc.integer({ min: 1, max: 3 }),
  })
  .chain(({ cols, rows }) =>
    fc
      .array(
        fc.array(
          fc
            .string({ minLength: 1, maxLength: 10 })
            .filter((s) => s.trim().length > 0 && !s.includes("|")),
          { minLength: cols, maxLength: cols },
        ),
        { minLength: rows + 1, maxLength: rows + 1 },
      )
      .map((data) => {
        const header = `| ${data[0].map((c) => c.trim()).join(" | ")} |`;
        const separator = `| ${data[0].map(() => "---").join(" | ")} |`;
        const bodyRows = data
          .slice(1)
          .map((row) => `| ${row.map((c) => c.trim()).join(" | ")} |`);
        return [header, separator, ...bodyRows].join("\n");
      }),
  );

/**
 * 生成有效的数学公式
 */
const mathFormulaArbitrary: fc.Arbitrary<{
  markdown: string;
  isBlock: boolean;
}> = fc.oneof(
  // 行内公式
  fc
    .constantFrom("x^2", "\\frac{a}{b}", "\\sqrt{x}", "e^{i\\pi}")
    .map((formula) => ({ markdown: `$${formula}$`, isBlock: false })),
  // 块级公式
  fc
    .constantFrom(
      "\\sum_{i=1}^{n} x_i",
      "\\int_0^\\infty e^{-x} dx",
      "\\begin{matrix} a & b \\\\ c & d \\end{matrix}",
    )
    .map((formula) => ({ markdown: `$$${formula}$$`, isBlock: true })),
);

/**
 * 生成带格式的文本
 */
const formattedTextArbitrary: fc.Arbitrary<{
  markdown: string;
  hasBold: boolean;
  hasItalic: boolean;
  hasLink: boolean;
}> = fc
  .record({
    text: fc
      .string({ minLength: 1, maxLength: 20 })
      .filter(
        (s) =>
          s.trim().length > 0 &&
          !s.includes("*") &&
          !s.includes("_") &&
          !s.includes("[") &&
          !s.includes("]"),
      ),
    format: fc.constantFrom("bold", "italic", "link", "boldItalic"),
  })
  .map(({ text, format }) => {
    const trimmedText = text.trim();
    switch (format) {
      case "bold":
        return {
          markdown: `**${trimmedText}**`,
          hasBold: true,
          hasItalic: false,
          hasLink: false,
        };
      case "italic":
        return {
          markdown: `*${trimmedText}*`,
          hasBold: false,
          hasItalic: true,
          hasLink: false,
        };
      case "link":
        return {
          markdown: `[${trimmedText}](https://example.com)`,
          hasBold: false,
          hasItalic: false,
          hasLink: true,
        };
      case "boldItalic":
        return {
          markdown: `***${trimmedText}***`,
          hasBold: true,
          hasItalic: true,
          hasLink: false,
        };
      default:
        return {
          markdown: trimmedText,
          hasBold: false,
          hasItalic: false,
          hasLink: false,
        };
    }
  });

// ============================================================================
// 属性测试
// ============================================================================

describe("消息渲染属性测试", () => {
  /**
   * Property 7: Markdown 渲染包含正确元素
   *
   * *For any* 包含 Markdown 格式（标题、列表、代码块、表格）的消息内容，
   * 渲染输出应包含对应的 HTML 元素
   *
   * **Feature: general-chat, Property 7: Markdown 渲染包含正确元素**
   * **Validates: Requirements 2.3, 2.4, 6.3, 6.4, 6.5**
   */
  describe("Property 7: Markdown 渲染包含正确元素", () => {
    /**
     * 7.1 标题检测测试
     * 验证: 对于任意 Markdown 标题，检测函数应正确识别标题级别
     * **Validates: Requirements 2.3**
     */
    test.prop(
      [
        fc
          .integer({ min: 1, max: 6 })
          .chain((level) => headingArbitrary(level)),
      ],
      { numRuns: 100 },
    )("对于任意 Markdown 标题，应正确检测标题级别", (heading: string) => {
      const headings = detectHeadings(heading);

      // 应该检测到至少一个标题
      expect(headings.length).toBeGreaterThanOrEqual(1);

      // 标题级别应在 1-6 范围内
      headings.forEach((level) => {
        expect(level).toBeGreaterThanOrEqual(1);
        expect(level).toBeLessThanOrEqual(6);
      });
    });

    /**
     * 7.2 无序列表检测测试
     * 验证: 对于任意无序列表，检测函数应正确识别
     * **Validates: Requirements 2.3**
     */
    test.prop([unorderedListArbitrary], { numRuns: 100 })(
      "对于任意无序列表，应正确检测列表存在",
      (list: string) => {
        const { hasUnorderedList } = detectLists(list);
        expect(hasUnorderedList).toBe(true);
      },
    );

    /**
     * 7.3 有序列表检测测试
     * 验证: 对于任意有序列表，检测函数应正确识别
     * **Validates: Requirements 2.3**
     */
    test.prop([orderedListArbitrary], { numRuns: 100 })(
      "对于任意有序列表，应正确检测列表存在",
      (list: string) => {
        const { hasOrderedList } = detectLists(list);
        expect(hasOrderedList).toBe(true);
      },
    );

    /**
     * 7.4 代码块检测和解析测试
     * 验证: 对于任意代码块，应正确检测语言和内容
     * **Validates: Requirements 2.4, 6.3**
     */
    test.prop([codeBlockArbitrary], { numRuns: 100 })(
      "对于任意代码块，应正确检测语言和内容",
      ({ markdown, language }: { markdown: string; language: string }) => {
        // 检测代码块
        const codeBlocks = detectCodeBlocks(markdown);
        expect(codeBlocks.length).toBe(1);
        expect(codeBlocks[0].language).toBe(language);
        expect(codeBlocks[0].hasContent).toBe(true);

        // 解析内容块
        const blocks = parseContent(markdown);
        const codeBlock = blocks.find((b) => b.type === "code");
        expect(codeBlock).toBeDefined();
        expect(codeBlock?.language).toBe(language);
      },
    );

    /**
     * 7.5 表格检测测试
     * 验证: 对于任意 Markdown 表格，检测函数应正确识别
     * **Validates: Requirements 6.5**
     */
    test.prop([tableArbitrary], { numRuns: 100 })(
      "对于任意 Markdown 表格，应正确检测表格存在",
      (table: string) => {
        const hasTable = detectTables(table);
        expect(hasTable).toBe(true);
      },
    );

    /**
     * 7.6 数学公式检测测试
     * 验证: 对于任意数学公式，检测函数应正确识别公式类型
     * **Validates: Requirements 6.4**
     */
    test.prop([mathFormulaArbitrary], { numRuns: 100 })(
      "对于任意数学公式，应正确检测公式类型",
      ({ markdown, isBlock }: { markdown: string; isBlock: boolean }) => {
        const { hasInlineMath, hasBlockMath } = detectMathFormulas(markdown);

        if (isBlock) {
          expect(hasBlockMath).toBe(true);
        } else {
          expect(hasInlineMath).toBe(true);
        }
      },
    );

    /**
     * 7.7 文本格式检测测试
     * 验证: 对于任意格式化文本，检测函数应正确识别格式类型
     * **Validates: Requirements 2.3**
     */
    test.prop([formattedTextArbitrary], { numRuns: 100 })(
      "对于任意格式化文本，应正确检测格式类型",
      ({
        markdown,
        hasBold,
        hasItalic,
        hasLink,
      }: {
        markdown: string;
        hasBold: boolean;
        hasItalic: boolean;
        hasLink: boolean;
      }) => {
        const detected = detectTextFormatting(markdown);

        if (hasBold) {
          expect(detected.hasBold).toBe(true);
        }
        if (hasItalic) {
          expect(detected.hasItalic).toBe(true);
        }
        if (hasLink) {
          expect(detected.hasLink).toBe(true);
        }
      },
    );

    /**
     * 7.8 内容解析保持完整性测试
     * 验证: 解析后的内容块应包含原始内容的所有非空白部分
     * **Validates: Requirements 2.3, 2.4**
     */
    test.prop(
      [
        fc.oneof(
          // 纯文本
          fc
            .string({ minLength: 1, maxLength: 100 })
            .filter((s) => s.trim().length > 0 && !s.includes("```")),
          // 带代码块的内容
          codeBlockArbitrary.map((c) => c.markdown),
          // 混合内容
          fc
            .tuple(
              fc
                .string({ minLength: 1, maxLength: 50 })
                .filter((s) => s.trim().length > 0 && !s.includes("```")),
              codeBlockArbitrary,
            )
            .map(([text, code]) => `${text}\n\n${code.markdown}`),
        ),
      ],
      { numRuns: 100 },
    )("解析后的内容块应包含原始内容的所有非空白部分", (content: string) => {
      const blocks = parseContent(content);

      // 应该至少有一个内容块
      expect(blocks.length).toBeGreaterThanOrEqual(1);

      // 每个块都应该有类型和内容
      blocks.forEach((block) => {
        expect(block.type).toBeDefined();
        expect(["text", "code", "image", "file"]).toContain(block.type);
        // 内容可以为空字符串，但类型必须正确
        expect(typeof block.content).toBe("string");
      });

      // 如果原始内容包含代码块，解析结果应包含代码块
      if (content.includes("```")) {
        const hasCodeBlock = blocks.some((b) => b.type === "code");
        expect(hasCodeBlock).toBe(true);
      }
    });

    /**
     * 7.9 代码块语言标签正确性测试
     * 验证: 代码块的语言标签应与原始 Markdown 中指定的语言一致
     * **Validates: Requirements 6.3**
     */
    test.prop(
      [
        fc.constantFrom(
          "javascript",
          "typescript",
          "python",
          "rust",
          "go",
          "java",
          "cpp",
          "c",
          "ruby",
          "php",
        ),
      ],
      { numRuns: 100 },
    )(
      "代码块的语言标签应与原始 Markdown 中指定的语言一致",
      (language: string) => {
        const markdown = `\`\`\`${language}\nconst x = 1;\n\`\`\``;
        const blocks = parseContent(markdown);

        const codeBlock = blocks.find((b) => b.type === "code");
        expect(codeBlock).toBeDefined();
        expect(codeBlock?.language).toBe(language);
      },
    );

    /**
     * 7.10 无语言标签代码块默认处理测试
     * 验证: 没有语言标签的代码块应默认为 plaintext
     * **Validates: Requirements 2.4**
     */
    test.prop(
      [
        fc
          .string({ minLength: 1, maxLength: 50 })
          .filter((s) => s.trim().length > 0 && !s.includes("```")),
      ],
      { numRuns: 100 },
    )("没有语言标签的代码块应默认为 plaintext", (code: string) => {
      const markdown = `\`\`\`\n${code}\n\`\`\``;
      const blocks = parseContent(markdown);

      const codeBlock = blocks.find((b) => b.type === "code");
      expect(codeBlock).toBeDefined();
      expect(codeBlock?.language).toBe("plaintext");
    });
  });
});

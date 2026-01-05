/**
 * @file API Key 脱敏属性测试
 * @description 测试 API Key 脱敏功能的正确性
 * @module lib/utils/apiKeyMask.test
 *
 * **Feature: proxycast-connect, Property 4: API Key Masking Format**
 * **Validates: Requirements 3.2**
 */

import { describe, expect, it } from "vitest";
import { test } from "@fast-check/vitest";
import * as fc from "fast-check";
import { maskApiKey } from "./apiKeyMask";

describe("API Key 脱敏", () => {
  /**
   * Property 4: API Key Masking Format
   *
   * *对于任意* API Key 字符串，脱敏后的版本应满足：
   * - 如果原始 Key 长度 > 8，显示前 4 位 + "..." + 后 4 位
   * - 如果原始 Key 长度 <= 8，返回 "****"
   *
   * **Validates: Requirements 3.2**
   */
  describe("Property 4: API Key Masking Format", () => {
    test.prop([fc.string({ minLength: 9, maxLength: 200 })], { numRuns: 100 })(
      "对于长度 > 8 的 Key，应显示前 4 位 + '...' + 后 4 位",
      (apiKey: string) => {
        const masked = maskApiKey(apiKey);

        // 验证格式：前4位 + ... + 后4位
        expect(masked).toMatch(/^.{4}\.\.\..{4}$/);

        // 验证前 4 位正确
        expect(masked.slice(0, 4)).toBe(apiKey.slice(0, 4));

        // 验证后 4 位正确
        expect(masked.slice(-4)).toBe(apiKey.slice(-4));

        // 验证中间是 "..."
        expect(masked.slice(4, 7)).toBe("...");

        // 验证总长度为 11（4 + 3 + 4）
        expect(masked.length).toBe(11);
      },
    );

    test.prop([fc.string({ minLength: 0, maxLength: 8 })], { numRuns: 100 })(
      "对于长度 <= 8 的 Key，应返回 '****'",
      (apiKey: string) => {
        const masked = maskApiKey(apiKey);
        expect(masked).toBe("****");
      },
    );

    // 边界测试：长度恰好为 8 和 9
    it("长度恰好为 8 的 Key 应返回 '****'", () => {
      const key8 = "12345678";
      expect(maskApiKey(key8)).toBe("****");
    });

    it("长度恰好为 9 的 Key 应显示前 4 后 4", () => {
      const key9 = "123456789";
      const masked = maskApiKey(key9);
      expect(masked).toBe("1234...6789");
    });

    // 空字符串测试
    it("空字符串应返回 '****'", () => {
      expect(maskApiKey("")).toBe("****");
    });

    // 典型 API Key 格式测试
    it("典型 OpenAI Key 格式应正确脱敏", () => {
      const key = "sk-1234567890abcdefghijklmnopqrstuvwxyz";
      const masked = maskApiKey(key);
      expect(masked).toBe("sk-1...wxyz");
    });

    it("典型 Anthropic Key 格式应正确脱敏", () => {
      const key = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz";
      const masked = maskApiKey(key);
      expect(masked).toBe("sk-a...wxyz");
    });
  });
});

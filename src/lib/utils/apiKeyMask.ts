/**
 * @file API Key 脱敏工具
 * @description 对 API Key 进行脱敏处理，保护敏感信息
 * @module lib/utils/apiKeyMask
 *
 * **Feature: proxycast-connect, Property 4: API Key Masking Format**
 * **Validates: Requirements 3.2**
 */

/**
 * 对 API Key 进行脱敏处理
 *
 * 规则：
 * - 如果 Key 长度 > 8，显示前 4 位 + "..." + 后 4 位
 * - 如果 Key 长度 <= 8，返回 "****"
 *
 * @param apiKey - 原始 API Key
 * @returns 脱敏后的字符串
 *
 * @example
 * maskApiKey("sk-1234567890abcdef") // "sk-1...cdef"
 * maskApiKey("short") // "****"
 * maskApiKey("12345678") // "****"
 * maskApiKey("123456789") // "1234...6789"
 */
export function maskApiKey(apiKey: string): string {
  if (apiKey.length <= 8) {
    return "****";
  }
  return `${apiKey.slice(0, 4)}...${apiKey.slice(-4)}`;
}

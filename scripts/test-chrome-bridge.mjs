#!/usr/bin/env node

/**
 * Chrome Bridge 测试脚本
 *
 * 使用方式：
 * 1. 确保 ProxyCast 服务器正在运行
 * 2. 确保 Chrome Profile 已打开并连接
 * 3. 运行: node scripts/test-chrome-bridge.mjs
 */

import WebSocket from 'ws';

const SERVER_URL = 'ws://127.0.0.1:8999';
const BRIDGE_KEY = 'Proxycast-key11';
const PROFILE_KEY = 'search_google';

// 连接 Control 通道
const controlUrl = `${SERVER_URL}/proxycast-chrome-control/${BRIDGE_KEY}`;
console.log(`[测试] 连接 Control 通道: ${controlUrl}`);

const ws = new WebSocket(controlUrl);

ws.on('open', async () => {
  console.log('[测试] ✅ Control 通道连接成功\n');

  // 测试 1: 获取页面信息
  console.log('=== 测试 1: 获取当前页面信息 ===');
  ws.send(JSON.stringify({
    type: 'command',
    request_id: 'test-1',
    profile_key: PROFILE_KEY,
    command: 'get_page_info',
    wait_for_page_info: true
  }));

  // 等待 3 秒
  await new Promise(resolve => setTimeout(resolve, 3000));

  // 测试 2: 打开 URL
  console.log('\n=== 测试 2: 打开 Google ===');
  ws.send(JSON.stringify({
    type: 'command',
    request_id: 'test-2',
    profile_key: PROFILE_KEY,
    command: 'open_url',
    url: 'https://www.google.com',
    wait_for_page_info: true
  }));

  // 等待 5 秒后关闭
  setTimeout(() => {
    console.log('\n[测试] 测试完成，关闭连接');
    ws.close();
  }, 8000);
});

ws.on('message', (data) => {
  try {
    const message = JSON.parse(data.toString());

    if (message.type === 'connection_ack') {
      console.log('[测试] 收到连接确认:', message.message);
      console.log('[测试] Client ID:', message.data?.clientId);
    } else if (message.type === 'command_result') {
      console.log(`\n[结果] Request ID: ${message.request_id}`);
      console.log(`[结果] 命令: ${message.command}`);
      console.log(`[结果] 成功: ${message.success}`);

      if (message.message) {
        console.log(`[结果] 消息: ${message.message}`);
      }

      if (message.error) {
        console.log(`[结果] 错误: ${message.error}`);
      }

      if (message.page_info) {
        console.log(`[结果] 页面标题: ${message.page_info.title}`);
        console.log(`[结果] 页面 URL: ${message.page_info.url}`);
        console.log(`[结果] Markdown 长度: ${message.page_info.markdown.length} 字符`);
        console.log(`[结果] Markdown 预览:\n${message.page_info.markdown.substring(0, 200)}...`);
      }
    } else if (message.type === 'heartbeat_ack') {
      // 忽略心跳响应
    } else {
      console.log('[测试] 收到消息:', message);
    }
  } catch (error) {
    console.error('[测试] 解析消息失败:', error.message);
    console.log('[测试] 原始消息:', data.toString());
  }
});

ws.on('error', (error) => {
  console.error('[测试] ❌ WebSocket 错误:', error.message);
});

ws.on('close', (code, reason) => {
  console.log(`\n[测试] 连接关闭: code=${code}, reason=${reason.toString()}`);
  process.exit(code === 1000 ? 0 : 1);
});

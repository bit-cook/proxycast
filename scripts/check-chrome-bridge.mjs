#!/usr/bin/env node

/**
 * 检查 Chrome Bridge 状态
 */

import WebSocket from 'ws';

const SERVER_URL = 'ws://127.0.0.1:8999';
const BRIDGE_KEY = 'Proxycast-key11';

// 连接 Observer 通道查看状态
const observerUrl = `${SERVER_URL}/proxycast-chrome-observer/${BRIDGE_KEY}?profileKey=test`;
console.log(`[检查] 连接 Observer 通道: ${observerUrl}`);

const ws = new WebSocket(observerUrl);

ws.on('open', () => {
  console.log('[检查] ✅ Observer 通道连接成功');
  console.log('[检查] 这说明服务器正常运行\n');

  setTimeout(() => {
    ws.close();
  }, 2000);
});

ws.on('message', (data) => {
  const message = JSON.parse(data.toString());
  console.log('[检查] 收到消息:', JSON.stringify(message, null, 2));
});

ws.on('error', (error) => {
  console.error('[检查] ❌ 错误:', error.message);
});

ws.on('close', () => {
  console.log('\n[检查] 连接关闭');
  process.exit(0);
});

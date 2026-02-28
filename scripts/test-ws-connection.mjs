#!/usr/bin/env node

import WebSocket from 'ws';

const serverUrl = 'ws://127.0.0.1:8999';
const bridgeKey = 'Proxycast-key11';
const profileKey = 'search_google';

const url = `${serverUrl}/proxycast-chrome-observer/${encodeURIComponent(bridgeKey)}?profileKey=${encodeURIComponent(profileKey)}`;

console.log(`[测试] 连接 URL: ${url}`);

const ws = new WebSocket(url);

ws.on('open', () => {
  console.log('[测试] ✅ WebSocket 连接成功');

  // 发送心跳
  const heartbeat = JSON.stringify({ type: 'heartbeat' });
  console.log(`[测试] 发送心跳: ${heartbeat}`);
  ws.send(heartbeat);

  setTimeout(() => {
    console.log('[测试] 关闭连接');
    ws.close();
  }, 2000);
});

ws.on('message', (data) => {
  console.log('[测试] 收到消息:', data.toString());
});

ws.on('error', (error) => {
  console.error('[测试] ❌ WebSocket 错误:', error.message);
});

ws.on('close', (code, reason) => {
  console.log(`[测试] 连接关闭: code=${code}, reason=${reason.toString()}`);
  process.exit(code === 1000 ? 0 : 1);
});

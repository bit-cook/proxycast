#!/usr/bin/env node

import { randomUUID } from 'node:crypto';
import process from 'node:process';

const DEFAULTS = {
  server: 'ws://127.0.0.1:8787',
  key: '',
  profile: 'default',
  timeoutMs: 15000,
};

function parseArgs(argv) {
  const args = { ...DEFAULTS };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--server' && argv[i + 1]) {
      args.server = argv[i + 1];
      i += 1;
      continue;
    }
    if (arg === '--key' && argv[i + 1]) {
      args.key = argv[i + 1];
      i += 1;
      continue;
    }
    if (arg === '--profile' && argv[i + 1]) {
      args.profile = argv[i + 1];
      i += 1;
      continue;
    }
    if (arg === '--timeout-ms' && argv[i + 1]) {
      args.timeoutMs = Number(argv[i + 1]);
      i += 1;
      continue;
    }
    if (arg === '--help' || arg === '-h') {
      printHelp();
      process.exit(0);
    }
  }
  return args;
}

function printHelp() {
  console.log(`
Proxycast Chrome Bridge E2E 联调脚本

用法:
  node scripts/chrome-bridge-e2e.mjs --key <proxycast_api_key> [选项]

选项:
  --server <ws_url>       服务地址，默认 ws://127.0.0.1:8787
  --key <api_key>         Proxycast API Key（必填）
  --profile <profile_key> profileKey，默认 default
  --timeout-ms <ms>       单步超时毫秒，默认 15000
  -h, --help              显示帮助

示例:
  node scripts/chrome-bridge-e2e.mjs --server ws://127.0.0.1:8787 --key proxy_cast --profile default
`);
}

function assertGlobalWebSocket() {
  if (typeof WebSocket !== 'undefined') {
    return;
  }
  throw new Error(
    '当前 Node 运行时不支持全局 WebSocket，请使用 Node 20+ 或安装支持 WebSocket 的运行环境。',
  );
}

function normalizeServer(server) {
  return String(server || '').trim().replace(/\/$/, '');
}

function toText(data) {
  if (typeof data === 'string') return data;
  if (Buffer.isBuffer(data)) return data.toString('utf8');
  if (data instanceof ArrayBuffer) return Buffer.from(data).toString('utf8');
  if (ArrayBuffer.isView(data)) return Buffer.from(data.buffer).toString('utf8');
  return String(data);
}

function createClient(url, label, timeoutMs) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const state = {
      label,
      ws,
      messages: [],
      waiters: [],
    };

    const timer = setTimeout(() => {
      reject(new Error(`[${label}] 连接超时: ${url}`));
      try {
        ws.close();
      } catch (_) {
        // ignore
      }
    }, timeoutMs);

    ws.onopen = () => {
      clearTimeout(timer);
      resolve(state);
    };

    ws.onerror = (event) => {
      clearTimeout(timer);
      reject(new Error(`[${label}] WebSocket 连接失败: ${event?.message || 'unknown error'}`));
    };

    ws.onmessage = (event) => {
      let payload;
      const text = toText(event.data);
      try {
        payload = JSON.parse(text);
      } catch (_) {
        payload = { type: 'raw_text', data: text };
      }
      state.messages.push(payload);

      const pending = [...state.waiters];
      for (const waiter of pending) {
        if (waiter.predicate(payload)) {
          waiter.resolve(payload);
          state.waiters = state.waiters.filter((item) => item !== waiter);
        }
      }
    };
  });
}

function waitForMessage(client, predicate, timeoutMs, desc) {
  const found = client.messages.find(predicate);
  if (found) {
    return Promise.resolve(found);
  }

  return new Promise((resolve, reject) => {
    const waiter = { predicate, resolve };
    client.waiters.push(waiter);
    const timer = setTimeout(() => {
      client.waiters = client.waiters.filter((item) => item !== waiter);
      reject(
        new Error(
          `[${client.label}] 等待消息超时(${timeoutMs}ms): ${desc}\n最近消息: ${JSON.stringify(
            client.messages.slice(-5),
            null,
            2,
          )}`,
        ),
      );
    }, timeoutMs);

    waiter.resolve = (payload) => {
      clearTimeout(timer);
      resolve(payload);
    };
  });
}

function send(client, payload) {
  client.ws.send(JSON.stringify(payload));
}

async function closeClient(client) {
  if (!client) return;
  await new Promise((resolve) => {
    try {
      client.ws.onclose = () => resolve();
      client.ws.close();
      setTimeout(resolve, 200);
    } catch (_) {
      resolve();
    }
  });
}

async function main() {
  assertGlobalWebSocket();
  const args = parseArgs(process.argv.slice(2));

  if (!args.key) {
    printHelp();
    throw new Error('缺少必填参数: --key');
  }
  if (!Number.isFinite(args.timeoutMs) || args.timeoutMs < 1000) {
    throw new Error('--timeout-ms 必须是 >= 1000 的数字');
  }

  const server = normalizeServer(args.server);
  const key = encodeURIComponent(args.key);
  const profile = encodeURIComponent(args.profile || 'default');
  const observerUrl = `${server}/proxycast-chrome-observer/Proxycast_Key=${key}?profileKey=${profile}`;
  const controlUrl = `${server}/proxycast-chrome-control/Proxycast_Key=${key}`;

  console.log('[E2E] observer:', observerUrl);
  console.log('[E2E] control :', controlUrl);

  let observer;
  let control;

  try {
    observer = await createClient(observerUrl, 'observer', args.timeoutMs);
    control = await createClient(controlUrl, 'control', args.timeoutMs);

    await waitForMessage(
      observer,
      (msg) => msg.type === 'connection_ack',
      args.timeoutMs,
      'observer connection_ack',
    );
    await waitForMessage(
      control,
      (msg) => msg.type === 'connection_ack',
      args.timeoutMs,
      'control connection_ack',
    );
    console.log('[E2E] 连接握手通过');

    send(observer, { type: 'heartbeat', timestamp: Date.now() });
    send(control, { type: 'heartbeat', timestamp: Date.now() });

    await waitForMessage(
      observer,
      (msg) => msg.type === 'heartbeat_ack',
      args.timeoutMs,
      'observer heartbeat_ack',
    );
    await waitForMessage(
      control,
      (msg) => msg.type === 'heartbeat_ack',
      args.timeoutMs,
      'control heartbeat_ack',
    );
    console.log('[E2E] 心跳通道通过');

    const requestId1 = `e2e-${randomUUID()}`;
    send(control, {
      type: 'command',
      data: {
        requestId: requestId1,
        command: 'get_page_info',
        wait_for_page_info: true,
      },
    });

    const cmdFromServer1 = await waitForMessage(
      observer,
      (msg) => msg.type === 'command' && msg.data?.requestId === requestId1,
      args.timeoutMs,
      'observer 收到 get_page_info 命令',
    );
    console.log('[E2E] observer 收到命令:', cmdFromServer1.data?.command);

    send(observer, {
      type: 'command_result',
      data: {
        requestId: requestId1,
        status: 'success',
        message: 'get_page_info executed by e2e observer',
      },
    });
    send(observer, {
      type: 'pageInfoUpdate',
      data: {
        markdown: '# E2E Page\nURL: https://example.com/e2e\n\n## 内容\nbridge e2e test',
      },
    });

    await waitForMessage(
      control,
      (msg) =>
        msg.type === 'command_result' &&
        msg.data?.requestId === requestId1 &&
        msg.data?.status === 'success',
      args.timeoutMs,
      'control 收到 command_result(success)',
    );
    await waitForMessage(
      control,
      (msg) =>
        msg.type === 'page_info_update' &&
        msg.data?.requestId === requestId1 &&
        typeof msg.data?.markdown === 'string' &&
        msg.data.markdown.includes('E2E Page'),
      args.timeoutMs,
      'control 收到 page_info_update',
    );
    console.log('[E2E] wait_for_page_info 命令链路通过');

    const requestId2 = `e2e-${randomUUID()}`;
    send(control, {
      type: 'command',
      data: {
        requestId: requestId2,
        command: 'scroll',
        text: 'down:300',
        wait_for_page_info: false,
      },
    });

    const cmdFromServer2 = await waitForMessage(
      observer,
      (msg) => msg.type === 'command' && msg.data?.requestId === requestId2,
      args.timeoutMs,
      'observer 收到 scroll 命令',
    );
    if (cmdFromServer2.data?.command !== 'scroll') {
      throw new Error(`期望 scroll，实际为 ${cmdFromServer2.data?.command || 'unknown'}`);
    }
    send(observer, {
      type: 'command_result',
      data: {
        requestId: requestId2,
        status: 'success',
        message: 'scroll executed by e2e observer',
      },
    });

    await waitForMessage(
      control,
      (msg) =>
        msg.type === 'command_result' &&
        msg.data?.requestId === requestId2 &&
        msg.data?.status === 'success',
      args.timeoutMs,
      'control 收到 scroll command_result',
    );
    console.log('[E2E] 非 wait_for_page_info 命令链路通过');

    console.log('\n[E2E] ✅ Chrome Bridge 联调通过');
  } finally {
    await closeClient(control);
    await closeClient(observer);
  }
}

main().catch((error) => {
  console.error('\n[E2E] ❌ Chrome Bridge 联调失败');
  console.error(error?.stack || error?.message || String(error));
  process.exit(1);
});

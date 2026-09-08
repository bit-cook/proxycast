import assert from "node:assert/strict";
import { once } from "node:events";

import { WebSocketServer } from "ws";
import { test } from "vitest";

import {
  AppServerClient,
  MAX_REMOTE_WEBSOCKET_MESSAGE_BYTES,
  connectRemoteAppServer,
  RemoteTransport,
} from "../dist/index.js";

function initializeResponse(
  id,
  name = "app-server",
  protocolVersion = "appserver.v0",
) {
  return {
    id,
    result: {
      serverInfo: { name, version: "test", protocolVersion },
      platform: { family: "unix", os: "test" },
      capabilities: {
        agentSession: true,
        capabilityDiscovery: true,
        artifact: true,
        evidence: true,
        workspace: true,
      },
    },
  };
}

async function listen(server) {
  await once(server, "listening");
  const address = server.address();
  assert.equal(typeof address, "object");
  return `ws://127.0.0.1:${address.port}/rpc`;
}

async function closeServer(server) {
  for (const client of server.clients) {
    client.close();
  }
  if (server.address()) {
    await new Promise((resolve) => server.close(resolve));
  }
}

test("remote App Server uses authenticated headers and completes the shared handshake", async () => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  const url = `${await listen(server)}?tenant=public`;
  const received = [];
  let initialized;
  let request;

  server.on("connection", (socket, httpRequest) => {
    assert.equal(httpRequest.headers.authorization, "Bearer local-test-token");
    assert.equal(httpRequest.headers["x-lime-tenant-id"], "tenant-42");
    assert.equal(httpRequest.url, "/rpc?tenant=public");
    assert.equal(httpRequest.url.includes("local-test-token"), false);
    socket.on("message", (data) => {
      const message = JSON.parse(data.toString());
      received.push(message);
      if (message.method === "initialize") {
        socket.send(JSON.stringify(initializeResponse(message.id)));
      } else if (message.method === "initialized") {
        initialized = message;
      } else if (message.method === "test/remote") {
        request = message;
        socket.send(JSON.stringify({ id: message.id, result: { ok: true } }));
      }
    });
  });

  const connected = await connectRemoteAppServer(
    {
      websocketUrl: url,
      authToken: "local-test-token",
      tenantId: "tenant-42",
    },
    { clientInfo: { name: "remote-test-client", version: "1" } },
  );
  const result = await connected.connection.request(
    connected.client.request("test/remote", { value: 1 }),
  );

  assert.deepEqual(result.result, { ok: true });
  assert.equal(received[0].method, "initialize");
  assert.equal(initialized.method, "initialized");
  assert.equal(request.method, "test/remote");
  assert.equal(connected.initializeResponse.serverInfo.name, "app-server");

  await connected.transport.close();
  await closeServer(server);
});

test("remote App Server rejects unsafe endpoint and credential configuration before dialing", async () => {
  await assert.rejects(
    () =>
      RemoteTransport.connect({
        websocketUrl: "ws://cloud.example/rpc",
        authToken: "secret",
      }),
    /wss:\/\//,
  );
  await assert.rejects(
    () =>
      RemoteTransport.connect({
        websocketUrl: "wss://user:password@cloud.example/rpc",
      }),
    /userinfo or a fragment/,
  );
  await assert.rejects(
    () =>
      RemoteTransport.connect({
        websocketUrl: "wss://cloud.example/rpc?access_token=secret",
      }),
    /credential query parameters/,
  );
  await assert.rejects(
    () =>
      RemoteTransport.connect({
        websocketUrl: "wss://cloud.example/rpc",
        authToken: "   ",
      }),
    /auth token must not be empty/,
  );
  await assert.rejects(
    () =>
      RemoteTransport.connect({
        websocketUrl: "wss://cloud.example/rpc",
        authToken: "secret\n",
      }),
    /invalid header characters/,
  );
  await assert.rejects(
    () =>
      RemoteTransport.connect({
        websocketUrl: "wss://cloud.example/rpc",
        tenantId: "\n",
      }),
    /invalid header characters/,
  );
});

test("remote App Server fails closed when identity or protocol does not match", async () => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  const url = await listen(server);
  const closed = new Promise((resolve) => {
    server.on("connection", (socket) => {
      socket.once("close", resolve);
      socket.once("message", (data) => {
        const initialize = JSON.parse(data.toString());
        socket.send(
          JSON.stringify(initializeResponse(initialize.id, "not-app-server")),
        );
      });
    });
  });

  await assert.rejects(
    () =>
      connectRemoteAppServer(
        { websocketUrl: url },
        { clientInfo: { name: "remote-test-client" } },
      ),
    /identity mismatch/,
  );
  await closed;
  await closeServer(server);
});

test("remote App Server rejects an unsupported protocol before initialized", async () => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  const url = await listen(server);
  const closed = new Promise((resolve) => {
    server.on("connection", (socket) => {
      socket.once("close", resolve);
      socket.once("message", (data) => {
        const initialize = JSON.parse(data.toString());
        socket.send(
          JSON.stringify(
            initializeResponse(initialize.id, "app-server", "appserver.v9"),
          ),
        );
      });
    });
  });

  await assert.rejects(
    () =>
      connectRemoteAppServer(
        { websocketUrl: url },
        { clientInfo: { name: "remote-test-client" } },
      ),
    /unsupported app-server protocol/,
  );
  await closed;
  await closeServer(server);
});

test("remote App Server transport rejects binary frames and enforces the configured message cap", async () => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  const url = await listen(server);
  const connectedPromise = new Promise((resolve) => {
    server.on("connection", (socket) => {
      socket.once("message", (data) => {
        const initialize = JSON.parse(data.toString());
        socket.send(JSON.stringify(initializeResponse(initialize.id)));
        socket.once("message", () => {
          resolve(socket);
          socket.send(Buffer.from("binary"));
        });
      });
    });
  });
  const connected = await connectRemoteAppServer(
    { websocketUrl: url, maxMessageBytes: 1_024 },
    { clientInfo: { name: "remote-test-client" } },
  );
  assert.throws(
    () =>
      connected.transport.send({
        id: 2,
        method: "test/oversized",
        params: { value: "x".repeat(2_048) },
      }),
    /exceeds 1024 bytes/,
  );
  await connectedPromise;
  await assert.rejects(
    () => connected.transport.nextMessage(1_000),
    /non-text WebSocket frame/,
  );
  assert.equal(MAX_REMOTE_WEBSOCKET_MESSAGE_BYTES, 128 * 1024 * 1024);

  await connected.transport.close();
  await closeServer(server);
});

test("remote handshake uses a caller-supplied AppServerClient request id", async () => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  const url = await listen(server);
  let initializeId;
  server.on("connection", (socket) => {
    socket.once("message", (data) => {
      const initialize = JSON.parse(data.toString());
      initializeId = initialize.id;
      socket.send(JSON.stringify(initializeResponse(initialize.id)));
    });
  });
  const client = new AppServerClient({ initialRequestId: 41 });
  const connected = await connectRemoteAppServer(
    { websocketUrl: url },
    { clientInfo: { name: "remote-test-client" } },
    { client },
  );
  assert.equal(initializeId, 41);
  await connected.transport.close();
  await closeServer(server);
});

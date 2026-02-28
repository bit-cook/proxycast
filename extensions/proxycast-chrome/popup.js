const DEFAULT_SETTINGS = {
  serverUrl: "ws://127.0.0.1:8999",
  bridgeKey: "",
  profileKey: "default",
  monitoringEnabled: true,
};

const bridgeStatusEl = document.getElementById("bridgeStatus");
const monitorStatusEl = document.getElementById("monitorStatus");
const endpointPreviewEl = document.getElementById("endpointPreview");

const serverUrlEl = document.getElementById("serverUrl");
const bridgeKeyEl = document.getElementById("bridgeKey");
const profileKeyEl = document.getElementById("profileKey");

const saveBtnEl = document.getElementById("saveBtn");
const toggleConnBtnEl = document.getElementById("toggleConnBtn");
const toggleMonitorBtnEl = document.getElementById("toggleMonitorBtn");
const captureBtnEl = document.getElementById("captureBtn");

const pageTitleEl = document.getElementById("pageTitle");
const pageUrlEl = document.getElementById("pageUrl");

function setBadge(el, isOn, onText, offText) {
  el.textContent = isOn ? onText : offText;
  el.className = `badge ${isOn ? "badge-on" : "badge-off"}`;
}

function buildObserverEndpoint(serverUrl, bridgeKey, profileKey) {
  const base = String(serverUrl || "").trim().replace(/\/$/, "");
  const key = String(bridgeKey || "").trim();
  const profile = encodeURIComponent(String(profileKey || "default").trim() || "default");
  if (!base || !key) {
    return "Observer URL: 未配置";
  }
  return `Observer URL: ${base}/proxycast-chrome-observer/Proxycast_Key=${encodeURIComponent(key)}?profileKey=${profile}`;
}

function applyStatus(status) {
  const connected = Boolean(status?.isConnected);
  const monitoring = Boolean(status?.monitoringEnabled);
  setBadge(bridgeStatusEl, connected, "已连接", "未连接");
  setBadge(monitorStatusEl, monitoring, "开启", "关闭");

  const latestPageInfo = status?.latestPageInfo;
  if (latestPageInfo?.title || latestPageInfo?.url) {
    pageTitleEl.textContent = latestPageInfo.title || "无标题";
    pageUrlEl.textContent = latestPageInfo.url || "";
  }

  const settings = status?.settings;
  if (settings) {
    if (typeof settings.serverUrl === "string" && settings.serverUrl) {
      serverUrlEl.value = settings.serverUrl;
    }
    if (typeof settings.profileKey === "string" && settings.profileKey) {
      profileKeyEl.value = settings.profileKey;
    }
    endpointPreviewEl.textContent = buildObserverEndpoint(
      settings.serverUrl,
      bridgeKeyEl.value,
      settings.profileKey,
    );
  }
}

function sendMessage(message) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(message, (response) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
        return;
      }
      resolve(response);
    });
  });
}

function readStoredSettings() {
  return new Promise((resolve) => {
    chrome.storage.local.get(Object.keys(DEFAULT_SETTINGS), (stored) => {
      resolve({ ...DEFAULT_SETTINGS, ...stored });
    });
  });
}

async function loadInitialState() {
  const settings = await readStoredSettings();
  serverUrlEl.value = settings.serverUrl;
  bridgeKeyEl.value = settings.bridgeKey;
  profileKeyEl.value = settings.profileKey;
  endpointPreviewEl.textContent = buildObserverEndpoint(
    settings.serverUrl,
    settings.bridgeKey,
    settings.profileKey,
  );

  try {
    const status = await sendMessage({ type: "GET_STATUS" });
    applyStatus(status || {});
  } catch (error) {
    console.warn("[ProxycastBridgePopup] 获取状态失败", error?.message || String(error));
  }
}

async function saveAndReconnect() {
  const payload = {
    serverUrl: serverUrlEl.value.trim(),
    bridgeKey: bridgeKeyEl.value.trim(),
    profileKey: profileKeyEl.value.trim() || "default",
    reconnect: true,
  };

  endpointPreviewEl.textContent = buildObserverEndpoint(
    payload.serverUrl,
    payload.bridgeKey,
    payload.profileKey,
  );

  saveBtnEl.disabled = true;
  const originalText = saveBtnEl.textContent;
  saveBtnEl.textContent = "保存中...";

  try {
    await sendMessage({ type: "UPDATE_SETTINGS", data: payload });
    saveBtnEl.textContent = "已保存";
    setTimeout(() => {
      saveBtnEl.textContent = originalText;
      saveBtnEl.disabled = false;
    }, 900);
  } catch (error) {
    saveBtnEl.textContent = "保存失败";
    setTimeout(() => {
      saveBtnEl.textContent = originalText;
      saveBtnEl.disabled = false;
    }, 1200);
    console.warn("[ProxycastBridgePopup] 保存设置失败", error?.message || String(error));
  }
}

async function toggleConnection() {
  try {
    await sendMessage({ type: "TOGGLE_CONNECTION" });
  } catch (error) {
    console.warn("[ProxycastBridgePopup] 切换连接失败", error?.message || String(error));
  }
}

async function toggleMonitoring() {
  try {
    await sendMessage({ type: "TOGGLE_MONITORING" });
  } catch (error) {
    console.warn("[ProxycastBridgePopup] 切换监控失败", error?.message || String(error));
  }
}

async function capturePageNow() {
  captureBtnEl.disabled = true;
  const originalText = captureBtnEl.textContent;
  captureBtnEl.textContent = "抓取中...";

  try {
    await sendMessage({ type: "REQUEST_PAGE_CAPTURE" });
  } catch (error) {
    console.warn("[ProxycastBridgePopup] 请求抓取失败", error?.message || String(error));
  } finally {
    setTimeout(() => {
      captureBtnEl.textContent = originalText;
      captureBtnEl.disabled = false;
    }, 800);
  }
}

async function pasteConfigFromClipboard() {
  const pasteBtn = document.getElementById("pasteConfigBtn");
  pasteBtn.disabled = true;
  const originalText = pasteBtn.textContent;
  pasteBtn.textContent = "粘贴中...";

  try {
    const text = await navigator.clipboard.readText();
    const config = JSON.parse(text);

    if (config.serverUrl) {
      serverUrlEl.value = config.serverUrl;
    }
    if (config.bridgeKey) {
      bridgeKeyEl.value = config.bridgeKey;
    }
    if (config.profileKey) {
      profileKeyEl.value = config.profileKey;
    }

    endpointPreviewEl.textContent = buildObserverEndpoint(
      serverUrlEl.value,
      bridgeKeyEl.value,
      profileKeyEl.value,
    );

    pasteBtn.textContent = "已粘贴";
    setTimeout(() => {
      pasteBtn.textContent = originalText;
      pasteBtn.disabled = false;
    }, 1000);
  } catch (error) {
    pasteBtn.textContent = "粘贴失败";
    setTimeout(() => {
      pasteBtn.textContent = originalText;
      pasteBtn.disabled = false;
    }, 1500);
    console.warn("[ProxycastBridgePopup] 粘贴配置失败", error?.message || String(error));
  }
}

function clearConfig() {
  serverUrlEl.value = DEFAULT_SETTINGS.serverUrl;
  bridgeKeyEl.value = "";
  profileKeyEl.value = DEFAULT_SETTINGS.profileKey;
  endpointPreviewEl.textContent = buildObserverEndpoint(
    serverUrlEl.value,
    bridgeKeyEl.value,
    profileKeyEl.value,
  );
}

saveBtnEl.addEventListener("click", saveAndReconnect);
toggleConnBtnEl.addEventListener("click", toggleConnection);
toggleMonitorBtnEl.addEventListener("click", toggleMonitoring);
captureBtnEl.addEventListener("click", capturePageNow);
document.getElementById("pasteConfigBtn").addEventListener("click", pasteConfigFromClipboard);
document.getElementById("clearConfigBtn").addEventListener("click", clearConfig);

for (const input of [serverUrlEl, bridgeKeyEl, profileKeyEl]) {
  input.addEventListener("input", () => {
    endpointPreviewEl.textContent = buildObserverEndpoint(
      serverUrlEl.value,
      bridgeKeyEl.value,
      profileKeyEl.value,
    );
  });
}

chrome.runtime.onMessage.addListener((request) => {
  if (request?.type === "STATUS_UPDATE") {
    applyStatus(request?.data || {});
  }
});

loadInitialState();

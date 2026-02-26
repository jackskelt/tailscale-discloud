const API = "/api";

// ─── State ───────────────────────────────────────────────────────────────
let tunnels = [];
let suggestions = [];
let i18nStrings = {};
let i18nLanguages = [];
let currentLang = "en";
let tailscaleHostname = "tailscale-discloud";

// ─── DOM Cache ───────────────────────────────────────────────────────────
const $ = (id) => document.getElementById(id);

const dom = {
  // Stats
  statTotal: $("stat-total"),
  statActive: $("stat-active"),
  statInactive: $("stat-inactive"),

  // Create form
  formTitle: $("form-title"),
  tunnelForm: $("tunnel-form"),
  inputName: $("input-name"),
  inputLocalPort: $("input-local-port"),
  inputTargetHost: $("input-target-host"),
  inputTargetPort: $("input-target-port"),
  inputEnabled: $("input-enabled"),
  formSubmitBtn: $("form-submit-btn"),
  formCancelBtn: $("form-cancel-btn"),

  // Suggestions
  suggestionsSection: $("suggestions-section"),
  suggestionsGrid: $("suggestions-grid"),

  // Tunnel table
  emptyState: $("empty-state"),
  tableContainer: $("table-container"),
  tunnelsTbody: $("tunnels-tbody"),

  // Test modal
  testOverlay: $("test-overlay"),
  testModalTitle: $("test-modal-title"),
  testModalClose: $("test-modal-close"),
  testModalDone: $("test-modal-done"),
  testResultContainer: $("test-result-container"),

  // Edit modal
  editOverlay: $("edit-overlay"),
  editModalClose: $("edit-modal-close"),
  editForm: $("edit-form"),
  editId: $("edit-id"),
  editName: $("edit-name"),
  editLocalPort: $("edit-local-port"),
  editTargetHost: $("edit-target-host"),
  editTargetPort: $("edit-target-port"),
  editEnabled: $("edit-enabled"),
  editCancelBtn: $("edit-cancel-btn"),
  editSubmitBtn: $("edit-submit-btn"),

  // Header controls
  refreshBtn: $("refresh-btn"),
  themeToggle: $("theme-toggle"),
  langSelector: $("lang-selector"),
  langBtn: $("lang-btn"),
  langDropdown: $("lang-dropdown"),
  langCurrentFlag: $("lang-current-flag"),
  langCurrentName: $("lang-current-name"),

  // Toasts
  toastContainer: $("toast-container"),
};

// ═══════════════════════════════════════════════════════════════════════════
// Initialization
// ═══════════════════════════════════════════════════════════════════════════

document.addEventListener("DOMContentLoaded", async () => {
  initTheme();
  await initI18n();
  bindEvents();
  loadConfig();
  loadSuggestions();
  loadTunnels();
});

// ═══════════════════════════════════════════════════════════════════════════
// Theme System
// ═══════════════════════════════════════════════════════════════════════════

function initTheme() {
  const stored = localStorage.getItem("theme");
  if (stored) {
    setTheme(stored);
  } else {
    const prefersDark = window.matchMedia(
      "(prefers-color-scheme: dark)",
    ).matches;
    setTheme(prefersDark ? "dark" : "light");
  }
}

function setTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("theme", theme);
}

function toggleTheme() {
  const current = document.documentElement.getAttribute("data-theme");
  setTheme(current === "dark" ? "light" : "dark");
}

// ═══════════════════════════════════════════════════════════════════════════
// i18n System
// ═══════════════════════════════════════════════════════════════════════════

async function initI18n() {
  try {
    const res = await fetch("/i18n/index.json");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    i18nLanguages = data.languages || [];
  } catch (err) {
    console.warn("[i18n] Could not load language index:", err);
    i18nLanguages = [{ code: "en", name: "English", flag: "🇺🇸" }];
  }

  const stored = localStorage.getItem("lang");
  if (stored && i18nLanguages.some((l) => l.code === stored)) {
    currentLang = stored;
  } else {
    currentLang = detectBrowserLang();
  }

  await loadLanguage(currentLang);
  renderLangDropdown();
  updateLangButton();
}

function detectBrowserLang() {
  const navLangs = navigator.languages || [navigator.language || "en"];

  for (const navLang of navLangs) {
    const exact = i18nLanguages.find(
      (l) => l.code.toLowerCase() === navLang.toLowerCase(),
    );
    if (exact) return exact.code;

    const prefix = navLang.split("-")[0].toLowerCase();
    const partial = i18nLanguages.find((l) =>
      l.code.toLowerCase().startsWith(prefix),
    );
    if (partial) return partial.code;
  }

  return "en";
}

async function loadLanguage(code) {
  try {
    const res = await fetch(`/i18n/${code}.json`);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    i18nStrings = await res.json();
    currentLang = code;
    localStorage.setItem("lang", code);
  } catch (err) {
    console.warn(`[i18n] Could not load ${code}, falling back to en:`, err);
    if (code !== "en") {
      await loadLanguage("en");
      return;
    }
    i18nStrings = {};
  }
  applyI18n();
}

/**
 * Translate a key, optionally substituting {param} placeholders.
 * Falls back to the key itself when no translation is found.
 */
function t(key, params = {}) {
  let str = i18nStrings[key] || key;
  for (const [k, v] of Object.entries(params)) {
    str = str.replace(new RegExp(`\\{${k}\\}`, "g"), v);
  }
  return str;
}

/**
 * Resolve an API error/warning message. The backend returns
 * `{ id: "api.error.xxx", params: { ... } }` — we pass it through the
 * i18n system.
 */
function resolveApiMessage(msg) {
  if (!msg) return "";
  if (typeof msg === "string") return msg;
  if (msg.id) {
    const p = {};
    if (msg.params) {
      for (const [k, v] of Object.entries(msg.params)) {
        p[k] = String(v);
      }
    }
    return t(msg.id, p);
  }
  return JSON.stringify(msg);
}

function applyI18n() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    const translation = i18nStrings[key];
    if (translation) el.textContent = translation;
  });

  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
    const key = el.getAttribute("data-i18n-placeholder");
    const translation = i18nStrings[key];
    if (translation) el.placeholder = translation;
  });

  document.querySelectorAll("[data-i18n-title]").forEach((el) => {
    const key = el.getAttribute("data-i18n-title");
    const translation = i18nStrings[key];
    if (translation) el.title = translation;
  });

  document.documentElement.lang = currentLang;

  if (tunnels.length > 0 || dom.tunnelsTbody) {
    renderTunnels();
  }
  if (suggestions.length > 0) {
    renderSuggestions();
  }
}

function renderLangDropdown() {
  if (!dom.langDropdown) return;
  dom.langDropdown.innerHTML = i18nLanguages
    .map(
      (lang) =>
        `<button class="lang-option ${lang.code === currentLang ? "active" : ""}" data-lang="${esc(lang.code)}">
      <span class="lang-flag">${esc(lang.flag)}</span>
      <span>${esc(lang.name)}</span>
    </button>`,
    )
    .join("");

  dom.langDropdown.querySelectorAll(".lang-option").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const code = btn.getAttribute("data-lang");
      await loadLanguage(code);
      updateLangButton();
      renderLangDropdown();
      closeLangDropdown();
    });
  });
}

function updateLangButton() {
  const lang = i18nLanguages.find((l) => l.code === currentLang);
  if (!lang) return;
  if (dom.langCurrentFlag) dom.langCurrentFlag.textContent = lang.flag;
  if (dom.langCurrentName)
    dom.langCurrentName.textContent = lang.code.toUpperCase().substring(0, 2);
}

function toggleLangDropdown() {
  dom.langSelector.classList.toggle("open");
}

function closeLangDropdown() {
  dom.langSelector.classList.remove("open");
}

// ═══════════════════════════════════════════════════════════════════════════
// Event Binding
// ═══════════════════════════════════════════════════════════════════════════

function bindEvents() {
  dom.tunnelForm.addEventListener("submit", onCreateSubmit);
  dom.formCancelBtn.addEventListener("click", resetCreateForm);

  dom.refreshBtn.addEventListener("click", loadTunnels);
  dom.themeToggle.addEventListener("click", toggleTheme);

  dom.langBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleLangDropdown();
  });

  document.addEventListener("click", (e) => {
    if (!dom.langSelector.contains(e.target)) {
      closeLangDropdown();
    }
  });

  dom.testModalClose.addEventListener("click", () =>
    closeOverlay(dom.testOverlay),
  );
  dom.testModalDone.addEventListener("click", () =>
    closeOverlay(dom.testOverlay),
  );
  dom.testOverlay.addEventListener("click", (e) => {
    if (e.target === dom.testOverlay) closeOverlay(dom.testOverlay);
  });

  dom.editModalClose.addEventListener("click", () =>
    closeOverlay(dom.editOverlay),
  );
  dom.editCancelBtn.addEventListener("click", () =>
    closeOverlay(dom.editOverlay),
  );
  dom.editOverlay.addEventListener("click", (e) => {
    if (e.target === dom.editOverlay) closeOverlay(dom.editOverlay);
  });
  dom.editForm.addEventListener("submit", onEditSubmit);

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      closeOverlay(dom.testOverlay);
      closeOverlay(dom.editOverlay);
      closeLangDropdown();
    }
  });
}

// ═══════════════════════════════════════════════════════════════════════════
// API Helpers
// ═══════════════════════════════════════════════════════════════════════════

async function api(path, opts = {}) {
  const url = `${API}${path}`;
  const config = {
    headers: { "Content-Type": "application/json" },
    ...opts,
  };

  const res = await fetch(url, config);

  if (res.status === 204) return null;

  const body = await res.json().catch(() => null);

  if (!res.ok) {
    // The backend returns { error: { id, params } }
    if (body && body.error) {
      const msg = resolveApiMessage(body.error);
      throw new Error(msg);
    }
    throw new Error(`HTTP ${res.status}`);
  }

  return body;
}

// ═══════════════════════════════════════════════════════════════════════════
// Data Loading
// ═══════════════════════════════════════════════════════════════════════════

async function loadConfig() {
  try {
    const cfg = await api("/config");
    if (cfg && cfg.hostname) {
      tailscaleHostname = cfg.hostname;
    }
  } catch (err) {
    console.warn("[config] Could not load config:", err);
  }
}

async function loadTunnels() {
  try {
    dom.refreshBtn.classList.add("spinning");
    tunnels = await api("/tunnels");
    renderTunnels();
    updateStats();
  } catch (err) {
    toast(t("toast.tunnel.loadFail", { error: err.message }), "error");
  } finally {
    dom.refreshBtn.classList.remove("spinning");
  }
}

async function loadSuggestions() {
  try {
    const res = await fetch("/suggestions.json");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    suggestions = await res.json();
    renderSuggestions();
  } catch (err) {
    console.warn("[suggestions] Could not load suggestions.json:", err);
    if (dom.suggestionsSection) {
      dom.suggestionsSection.style.display = "none";
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Create Tunnel
// ═══════════════════════════════════════════════════════════════════════════

async function onCreateSubmit(e) {
  e.preventDefault();

  const data = readCreateForm();
  if (!validateTunnelData(data)) return;

  const btn = dom.formSubmitBtn;
  btn.disabled = true;
  const btnSvg = btn.querySelector("svg");
  const spinner = document.createElement("div");
  spinner.className = "spinner spinner-sm";
  if (btnSvg) {
    btnSvg.style.display = "none";
    btn.insertBefore(spinner, btnSvg);
  } else {
    btn.prepend(spinner);
  }

  try {
    const result = await api("/tunnels", {
      method: "POST",
      body: JSON.stringify(data),
    });

    // The backend returns a TunnelResponse with { ...tunnel, connection_url, warning }
    const tunnel = extractTunnel(result);
    tunnels.push(tunnel);
    renderTunnels();
    updateStats();
    resetCreateForm();
    toast(t("toast.tunnel.created", { name: tunnel.name }));

    // Show warning if present
    if (result.warning) {
      toast(resolveApiMessage(result.warning), "warning");
    }

    // Auto-test the connection after creation
    if (tunnel.enabled) {
      autoTestTunnel(tunnel);
    }
  } catch (err) {
    toast(t("toast.tunnel.createFail", { error: err.message }), "error");
  } finally {
    spinner.remove();
    if (btnSvg) btnSvg.style.display = "";
    btn.disabled = false;
  }
}

function readCreateForm() {
  return {
    name: dom.inputName.value.trim(),
    local_port: parseInt(dom.inputLocalPort.value, 10),
    target_host: dom.inputTargetHost.value.trim(),
    target_port: parseInt(dom.inputTargetPort.value, 10),
    enabled: dom.inputEnabled.checked,
  };
}

function resetCreateForm() {
  dom.tunnelForm.reset();
  dom.inputEnabled.checked = true;
  dom.formCancelBtn.style.display = "none";

  const btnSpan = dom.formSubmitBtn.querySelector("[data-i18n]");
  if (btnSpan) {
    btnSpan.textContent = t("form.btn.create");
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Auto-Test After Creation
// ═══════════════════════════════════════════════════════════════════════════

async function autoTestTunnel(tunnel) {
  try {
    const result = await api("/test", {
      method: "POST",
      body: JSON.stringify({
        target_host: tunnel.target_host,
        target_port: tunnel.target_port,
      }),
    });

    if (result.success) {
      toast(t("test.auto.success", { name: tunnel.name }), "success");
    } else {
      toast(
        t("test.auto.failure", {
          name: tunnel.name,
          host: tunnel.target_host,
          port: tunnel.target_port,
        }),
        "warning",
      );
    }
  } catch (err) {
    toast(
      t("test.auto.failure", {
        name: tunnel.name,
        host: tunnel.target_host,
        port: tunnel.target_port,
      }),
      "warning",
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Edit Tunnel (Modal)
// ═══════════════════════════════════════════════════════════════════════════

function openEditModal(id) {
  const tunnel = tunnels.find((t) => t.id === id);
  if (!tunnel) return;

  dom.editId.value = tunnel.id;
  dom.editName.value = tunnel.name;
  dom.editLocalPort.value = tunnel.local_port;
  dom.editTargetHost.value = tunnel.target_host;
  dom.editTargetPort.value = tunnel.target_port;
  dom.editEnabled.checked = tunnel.enabled;

  openOverlay(dom.editOverlay);
}

async function onEditSubmit(e) {
  e.preventDefault();

  const id = dom.editId.value;
  const data = {
    name: dom.editName.value.trim(),
    local_port: parseInt(dom.editLocalPort.value, 10),
    target_host: dom.editTargetHost.value.trim(),
    target_port: parseInt(dom.editTargetPort.value, 10),
    enabled: dom.editEnabled.checked,
  };

  if (!validateTunnelData(data)) return;

  const btn = dom.editSubmitBtn;
  btn.disabled = true;
  const spinner = document.createElement("div");
  spinner.className = "spinner spinner-sm";
  btn.prepend(spinner);

  try {
    const result = await api(`/tunnels/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });

    const updated = extractTunnel(result);
    const idx = tunnels.findIndex((t) => t.id === id);
    if (idx !== -1) tunnels[idx] = updated;

    renderTunnels();
    updateStats();
    closeOverlay(dom.editOverlay);
    toast(t("toast.tunnel.updated", { name: updated.name }));

    if (result.warning) {
      toast(resolveApiMessage(result.warning), "warning");
    }
  } catch (err) {
    toast(t("toast.tunnel.updateFail", { error: err.message }), "error");
  } finally {
    spinner.remove();
    btn.disabled = false;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Toggle Tunnel
// ═══════════════════════════════════════════════════════════════════════════

async function toggleTunnel(id) {
  const tunnel = tunnels.find((t) => t.id === id);
  if (!tunnel) return;

  try {
    const result = await api(`/tunnels/${id}`, {
      method: "PUT",
      body: JSON.stringify({ enabled: !tunnel.enabled }),
    });

    const updated = extractTunnel(result);
    const idx = tunnels.findIndex((t) => t.id === id);
    if (idx !== -1) tunnels[idx] = updated;

    renderTunnels();
    updateStats();

    const msgKey = updated.enabled
      ? "toast.tunnel.enabled"
      : "toast.tunnel.disabled";
    toast(t(msgKey, { name: updated.name }));

    if (result.warning) {
      toast(resolveApiMessage(result.warning), "warning");
    }
  } catch (err) {
    toast(t("toast.tunnel.toggleFail", { error: err.message }), "error");
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Delete Tunnel
// ═══════════════════════════════════════════════════════════════════════════

async function deleteTunnel(id) {
  const tunnel = tunnels.find((t) => t.id === id);
  const name = tunnel ? tunnel.name : id;

  if (!confirm(t("confirm.delete", { name }))) {
    return;
  }

  try {
    await api(`/tunnels/${id}`, { method: "DELETE" });
    tunnels = tunnels.filter((t) => t.id !== id);
    renderTunnels();
    updateStats();
    toast(t("toast.tunnel.deleted", { name }));
  } catch (err) {
    toast(t("toast.tunnel.deleteFail", { error: err.message }), "error");
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test Connection
// ═══════════════════════════════════════════════════════════════════════════

async function testConnection(targetHost, targetPort, btnEl) {
  if (btnEl) {
    btnEl.disabled = true;
    btnEl.innerHTML = '<span class="spinner spinner-sm"></span>';
  }

  const titleSpan = dom.testModalTitle.querySelector("[data-i18n]");
  if (titleSpan) {
    titleSpan.textContent = t("test.modal.testing", {
      host: targetHost,
      port: targetPort,
    });
  }

  dom.testResultContainer.innerHTML = `
    <div style="text-align:center; padding: var(--space-xl);">
      <div class="spinner"></div>
      <p class="text-muted" style="margin-top: var(--space-md);">${esc(t("test.modal.connecting"))}</p>
    </div>`;
  openOverlay(dom.testOverlay);

  try {
    const result = await api("/test", {
      method: "POST",
      body: JSON.stringify({
        target_host: targetHost,
        target_port: targetPort,
      }),
    });

    const statusClass = result.success ? "success" : "failure";
    const statusIcon = result.success ? "✓" : "✕";
    const statusText = result.success
      ? t("test.status.success")
      : t("test.status.failure");

    if (titleSpan) {
      titleSpan.textContent = t("test.modal.result", {
        host: targetHost,
        port: targetPort,
      });
    }

    dom.testResultContainer.innerHTML = `
      <div class="test-status ${statusClass}">
        <span class="test-status-icon">${statusIcon}</span>
        <span>${esc(statusText)}</span>
      </div>
      <p class="test-output-label">${esc(t("test.label.output"))}</p>
      <pre class="test-output">${esc(result.log)}</pre>`;
  } catch (err) {
    dom.testResultContainer.innerHTML = `
      <div class="test-status failure">
        <span class="test-status-icon">✕</span>
        <span>${esc(t("test.status.failure"))}: ${esc(err.message)}</span>
      </div>`;
  } finally {
    if (btnEl) {
      btnEl.disabled = false;
      btnEl.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>`;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Suggestions
// ═══════════════════════════════════════════════════════════════════════════

function applySuggestion(index) {
  const s = suggestions[index];
  if (!s) return;

  resetCreateForm();
  dom.inputName.value = s.name;
  dom.inputTargetHost.value = s.target_host;
  dom.inputTargetPort.value = s.target_port;
  dom.inputLocalPort.value = s.target_port;
  dom.inputLocalPort.focus();
  dom.inputLocalPort.select();

  toast(t("toast.suggestion.applied", { name: s.name }), "info");
  dom.tunnelForm.scrollIntoView({ behavior: "smooth", block: "center" });
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Extract the flat tunnel object from a TunnelResponse (which may have
 * flattened tunnel fields alongside `connection_url` and `warning`).
 * We keep `connection_url` on the tunnel for rendering.
 */
function extractTunnel(response) {
  if (!response) return response;
  // The backend flattens tunnel fields into the response root,
  // so we just pass through and keep connection_url.
  return response;
}

/**
 * Compute a connection URL client-side for list items that might not
 * have it (fallback).
 */
function getConnectionUrl(tun) {
  if (tun.connection_url) return tun.connection_url;
  if (tun.enabled) return `${tailscaleHostname}:${tun.local_port}`;
  return null;
}

/**
 * Copy text to clipboard and show toast feedback.
 */
async function copyToClipboard(text, btnEl) {
  try {
    await navigator.clipboard.writeText(text);

    // Visual feedback on the button
    if (btnEl) {
      btnEl.classList.add("copied");
      const orig = btnEl.innerHTML;
      btnEl.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>`;
      setTimeout(() => {
        btnEl.classList.remove("copied");
        btnEl.innerHTML = orig;
      }, 1500);
    }

    toast(t("toast.copied"), "success");
  } catch (err) {
    // Fallback: select text in a temporary input
    const input = document.createElement("input");
    input.value = text;
    document.body.appendChild(input);
    input.select();
    document.execCommand("copy");
    document.body.removeChild(input);
    toast(t("toast.copied"), "success");
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Rendering
// ═══════════════════════════════════════════════════════════════════════════

function renderTunnels() {
  const tbody = dom.tunnelsTbody;

  if (tunnels.length === 0) {
    tbody.innerHTML = "";
    dom.emptyState.style.display = "flex";
    dom.tableContainer.style.display = "none";
    return;
  }

  dom.emptyState.style.display = "none";
  dom.tableContainer.style.display = "block";

  tbody.innerHTML = tunnels
    .map((tun) => {
      const online = tun.enabled;
      const badgeClass = online ? "badge-online" : "badge-offline";
      const badgeText = online
        ? t("tunnels.status.online")
        : t("tunnels.status.offline");
      const toggleTitle = online
        ? t("actions.toggle.disable")
        : t("actions.toggle.enable");
      const toggleIcon = online
        ? `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>`
        : `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>`;

      const connUrl = getConnectionUrl(tun);
      const connUrlHtml = connUrl
        ? `<div class="connection-url-cell">
            <code class="connection-url-text">${esc(connUrl)}</code>
            <button class="btn-copy" onclick="event.stopPropagation(); copyToClipboard('${escAttr(connUrl)}', this)" title="${escAttr(t("actions.copy"))}">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
            </button>
          </div>`
        : `<span class="text-muted">—</span>`;

      // Build warning tooltip for persisted warnings (e.g. port closed)
      const warningHtml = tun.warning_id
        ? `<span class="tunnel-warning" title="${escAttr(t(tun.warning_id, { host: tun.target_host, port: tun.target_port }))}">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          </span>`
        : "";

      return `
        <tr>
          <td data-label="${escAttr(t("tunnels.col.status"))}">
            <div class="status-with-warning">
              <span class="badge ${badgeClass}">
                <span class="badge-dot"></span>
                ${esc(badgeText)}
              </span>
              ${warningHtml}
            </div>
          </td>
          <td data-label="${escAttr(t("tunnels.col.name"))}">
            <div class="tunnel-name-group">
              <div class="tunnel-name">${esc(tun.name)}</div>
              <div class="tunnel-id">${esc(tun.id.substring(0, 8))}…</div>
            </div>
          </td>
          <td data-label="${escAttr(t("tunnels.col.localPort"))}">
            <span class="tunnel-port">${tun.local_port}</span>
          </td>
          <td data-label="${escAttr(t("tunnels.col.target"))}">
            <span class="tunnel-endpoint">${esc(tun.target_host)}:${tun.target_port}</span>
          </td>
          <td data-label="${escAttr(t("tunnels.col.connection"))}" class="connection-url-td">
            ${connUrlHtml}
          </td>
          <td data-label="${escAttr(t("tunnels.col.actions"))}" style="text-align:right;">
            <div class="actions-cell">
              <button class="btn btn-ghost btn-icon" onclick="toggleTunnel('${tun.id}')" title="${escAttr(toggleTitle)}">
                ${toggleIcon}
              </button>
              <button class="btn btn-ghost btn-icon" onclick="testConnection('${escAttr(tun.target_host)}', ${tun.target_port}, this)" title="${escAttr(t("actions.test"))}">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
              </button>
              <button class="btn btn-ghost btn-icon" onclick="openEditModal('${tun.id}')" title="${escAttr(t("actions.edit"))}">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
              </button>
              <button class="btn btn-danger btn-icon" onclick="deleteTunnel('${tun.id}')" title="${escAttr(t("actions.delete"))}">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
              </button>
            </div>
          </td>
        </tr>`;
    })
    .join("");
}

function renderSuggestions() {
  if (!dom.suggestionsGrid || suggestions.length === 0) {
    if (dom.suggestionsSection) dom.suggestionsSection.style.display = "none";
    return;
  }

  if (dom.suggestionsSection) dom.suggestionsSection.style.display = "block";

  dom.suggestionsGrid.innerHTML = suggestions
    .map(
      (s, i) => `
      <div class="suggestion-card" onclick="applySuggestion(${i})" title="${escAttr(s.description)}">
        <span class="suggestion-name">${esc(s.name)}</span>
        <span class="suggestion-port">:${s.target_port}</span>
        <span class="suggestion-desc">${esc(s.description)}</span>
      </div>`,
    )
    .join("");
}

function updateStats() {
  const total = tunnels.length;
  const active = tunnels.filter((t) => t.enabled).length;
  const inactive = total - active;

  dom.statTotal.textContent = total;
  dom.statActive.textContent = active;
  dom.statInactive.textContent = inactive;
}

// ═══════════════════════════════════════════════════════════════════════════
// Validation
// ═══════════════════════════════════════════════════════════════════════════

function validateTunnelData(data) {
  if (!data.name) {
    toast(t("validation.nameRequired"), "error");
    return false;
  }
  if (!data.target_host) {
    toast(t("validation.targetHostRequired"), "error");
    return false;
  }
  if (!data.local_port || data.local_port < 1 || data.local_port > 65535) {
    toast(t("validation.localPortRange"), "error");
    return false;
  }
  if (!data.target_port || data.target_port < 1 || data.target_port > 65535) {
    toast(t("validation.targetPortRange"), "error");
    return false;
  }
  return true;
}

// ═══════════════════════════════════════════════════════════════════════════
// Modal Helpers
// ═══════════════════════════════════════════════════════════════════════════

function openOverlay(overlay) {
  overlay.classList.add("active");
  document.body.style.overflow = "hidden";
}

function closeOverlay(overlay) {
  overlay.classList.remove("active");
  const anyOpen = document.querySelector(".modal-overlay.active");
  if (!anyOpen) {
    document.body.style.overflow = "";
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Toast Notifications
// ═══════════════════════════════════════════════════════════════════════════

function toast(message, type = "success") {
  const el = document.createElement("div");
  el.className = `toast toast-${type}`;

  const icons = {
    success: "✓",
    error: "✕",
    info: "ℹ",
    warning: "⚠",
  };

  el.innerHTML = `<span class="toast-icon">${icons[type] || icons.info}</span><span>${esc(message)}</span>`;

  dom.toastContainer.appendChild(el);

  requestAnimationFrame(() => (el.style.opacity = "1"));

  setTimeout(() => {
    el.classList.add("toast-out");
    el.addEventListener("animationend", () => el.remove());
  }, 5000);
}

// ═══════════════════════════════════════════════════════════════════════════
// Escape Helpers
// ═══════════════════════════════════════════════════════════════════════════

function esc(str) {
  if (str == null) return "";
  const div = document.createElement("div");
  div.textContent = String(str);
  return div.innerHTML;
}

function escAttr(str) {
  if (str == null) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/'/g, "&#39;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

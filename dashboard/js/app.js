/**
 * HOSHIZORA SCADA Dashboard  —  app.js
 * Connects to HSZR-SCADA WebSocket, drives all live UI updates.
 */

const SCADA_HOST = '192.168.10.10';
const SCADA_PORT = 8000;
const WS_URL     = `ws://${SCADA_HOST}:${SCADA_PORT}/ws`;
const API_BASE   = `http://${SCADA_HOST}:${SCADA_PORT}`;

const MAX_LOG_LINES  = 120;
const MAX_ALERT_ROWS = 80;

// ─── Register metadata ────────────────────────────────────────────────────────
const REGISTER_META = {
  PUMP_CONTROL:   { label: 'PUMP',        unit: '',      icon: '⚙',  type: 'switch' },
  VALVE_STATUS:   { label: 'VALVE',       unit: '',      icon: '🔧', type: 'switch' },
  ALARM_OVERRIDE: { label: 'ALARM',       unit: '',      icon: '🔔', type: 'alarm'  },
  TEMPERATURE:    { label: 'TEMPERATURE', unit: '°C',    icon: '🌡', type: 'sensor', scale: 0.1 },
  PRESSURE:       { label: 'PRESSURE',    unit: ' kPa',  icon: '📊', type: 'sensor', scale: 0.1 },
};

// Register address map (1-based) for command API
const REG_ADDR = {
  PUMP_CONTROL: 1, VALVE_STATUS: 2,  ALARM_OVERRIDE: 3,
  TEMPERATURE: 4,   PRESSURE: 5,
};

// ─── State ────────────────────────────────────────────────────────────────────
let ws           = null;
let reconnectTmr = null;
let alertCount   = 0;
let logCount     = 0;
let isConnected  = false;

// ─── DOM refs ─────────────────────────────────────────────────────────────────
const els = {
  statusBadge:  document.getElementById('ws-status'),
  statusDot:    document.getElementById('ws-dot'),
  statusTxt:    document.getElementById('ws-label'),
  clock:        document.getElementById('clock'),
  alertCount:   document.getElementById('alert-count'),
  alertFeed:    document.getElementById('alert-feed'),
  logFeed:      document.getElementById('log-feed'),
  alertBadge:   document.getElementById('alert-panel-badge'),
};

// ─── Clock ────────────────────────────────────────────────────────────────────
function updateClock() {
  const now = new Date();
  els.clock.textContent = now.toLocaleTimeString('en-GB', { hour12: false });
}
setInterval(updateClock, 1000);
updateClock();

// ─── WebSocket ────────────────────────────────────────────────────────────────
function connect() {
  if (ws) { ws.onclose = null; ws.close(); }

  setConnected(false);
  ws = new WebSocket(WS_URL);

  ws.onopen = () => {
    clearTimeout(reconnectTmr);
    setConnected(true);
    appendLog('[WS] Connected to HSZR-SCADA', 'info');
  };

  ws.onmessage = (e) => {
    try { handleMessage(JSON.parse(e.data)); }
    catch (err) { console.warn('WS parse error', err); }
  };

  ws.onclose = () => {
    setConnected(false);
    appendLog('[WS] Connection lost — reconnecting in 3s…', 'err');
    reconnectTmr = setTimeout(connect, 3000);
  };

  ws.onerror = () => {
    appendLog('[WS] Socket error', 'err');
  };
}

function setConnected(val) {
  isConnected = val;
  els.statusBadge.className = 'status-badge' + (val ? ' connected' : ' error');
  els.statusTxt.textContent = val ? 'CONNECTED' : 'DISCONNECTED';
}

// ─── Message handling ─────────────────────────────────────────────────────────
function handleMessage(msg) {
  switch (msg.type) {
    case 'init':
      if (msg.data.registers) updateRegisters(msg.data.registers);
      if (msg.data.logs)      msg.data.logs.forEach(l => appendLog(l.message));
      if (msg.data.alerts)    msg.data.alerts.forEach(addAlert);
      break;
    case 'register_update':
      updateRegisters(msg.data.registers);
      break;
    case 'alert':
      addAlert(msg.data);
      break;
    case 'scada_log':
      appendLog(msg.data.message);
      break;
    case 'ping':
      break;
  }
}

// ─── Register card updates ────────────────────────────────────────────────────
function updateRegisters(regs) {
  for (const [name, raw] of Object.entries(regs)) {
    const meta = REGISTER_META[name];
    if (!meta) continue;
    const card  = document.getElementById(`reg-${name}`);
    const valEl = document.getElementById(`val-${name}`);
    if (!card || !valEl) continue;

    let display, stateClass;

    if (meta.type === 'switch') {
      display    = raw ? 'ON' : 'OFF';
      stateClass = raw ? 'active' : '';
    } else if (meta.type === 'alarm') {
      display    = raw ? 'ALARM' : 'NORMAL';
      stateClass = raw ? 'alarm' : '';
    } else {
      display    = ((raw * (meta.scale || 1)).toFixed(1)) + meta.unit;
      const danger = name === 'TEMPERATURE' ? raw > 500 : raw > 2000;
      stateClass = danger ? 'danger' : '';
    }

    valEl.textContent = display;
    card.className    = 'reg-card ' + stateClass;
  }
}

// ─── Alerts ───────────────────────────────────────────────────────────────────
function addAlert(event) {
  alertCount++;
  els.alertCount.textContent  = alertCount;
  els.alertBadge.textContent  = `${alertCount} TOTAL`;

  const noMsg = els.alertFeed.querySelector('.no-alerts');
  if (noMsg) noMsg.remove();

  const alert  = event.alert || {};
  const sig    = alert.signature || 'Unknown Signature';
  const src    = event.src_ip   || '?';
  const ts     = (event.timestamp || new Date().toISOString()).slice(11, 19);
  const sid    = alert.signature_id || '';

  const item = document.createElement('div');
  item.className = 'alert-item';
  item.innerHTML = `
    <div class="alert-icon">⚠️</div>
    <div class="alert-body">
      <div class="alert-sig" title="${sig}">${sig}</div>
      <div class="alert-meta">${ts}  ·  src: ${src}  ·  sid:${sid}</div>
    </div>`;

  els.alertFeed.prepend(item);

  // Trim excess
  while (els.alertFeed.children.length > MAX_ALERT_ROWS) {
    els.alertFeed.removeChild(els.alertFeed.lastChild);
  }

  // Flash header
  document.getElementById('alert-panel').style.boxShadow = '0 0 20px rgba(255,63,63,0.4)';
  setTimeout(() => { document.getElementById('alert-panel').style.boxShadow = ''; }, 1200);
}

// ─── Log feed ─────────────────────────────────────────────────────────────────
function appendLog(msg, cls = '') {
  logCount++;
  const now   = new Date().toLocaleTimeString('en-GB', { hour12: false });
  const line  = document.createElement('div');
  const extra = msg.includes('[ERROR]') ? 'err'
              : msg.includes('[WARN]')  ? 'warn' : cls;
  line.className   = 'log-line ' + extra;
  line.innerHTML   = `<span class="ts">${now}</span>${escHtml(msg)}`;
  els.logFeed.prepend(line);
  while (els.logFeed.children.length > MAX_LOG_LINES) {
    els.logFeed.removeChild(els.logFeed.lastChild);
  }
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

// ─── Attack buttons ───────────────────────────────────────────────────────────
async function fireAttack(label, register, value) {
  appendLog(`[SCADA CMD] ${label} → reg=${register} val=${value}`, 'warn');
  try {
    const res = await fetch(`${API_BASE}/command`, {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ register, value }),
    });
    const data = await res.json();
    appendLog(data.success ? `[CMD OK] ${label}` : `[CMD FAIL] ${label}`, data.success ? '' : 'err');
  } catch (e) {
    appendLog(`[CMD ERROR] ${e.message}`, 'err');
  }
}

// Attach to buttons defined in HTML
document.querySelectorAll('[data-reg]').forEach(btn => {
  btn.addEventListener('click', () => {
    const name  = btn.dataset.label  || btn.textContent;
    const reg   = parseInt(btn.dataset.reg);
    const val   = parseInt(btn.dataset.val);
    fireAttack(name, reg, val);
  });
});

// ─── Boot ─────────────────────────────────────────────────────────────────────
connect();

const POLL_MS = 1500;

const el = {
  clock: document.getElementById("clock"),
  status: document.getElementById("status"),
  map: document.getElementById("map"),
  feed: document.getElementById("feed"),
  detail: document.getElementById("agent-detail"),
  tooltip: document.getElementById("tooltip"),
};

let lastId = 0;
let selectedId = null;
let locations = [];
const agentEls = new Map();

function setStatus(text, ok) {
  el.status.textContent = text;
  el.status.className = "status " + (ok ? "ok" : "err");
}

async function poll() {
  try {
    const [state, events] = await Promise.all([
      fetch("/api/state").then((r) => r.json()),
      fetch("/api/events?after=" + lastId).then((r) => r.json()),
    ]);
    setStatus("live", true);
    lastState = state;
    renderClock(state);
    renderMap(state);
    if (selectedId != null) {
      const sel = state.agents.find((a) => a.id === selectedId);
      if (sel) renderDetail(sel);
    }
    if (events.length) {
      lastId = events[events.length - 1].id;
      appendEvents(events);
    }
  } catch (e) {
    setStatus("offline — retrying…", false);
  }
}

function renderClock(state) {
  el.clock.textContent = `Day ${state.day} · ${state.time}`;
  locations = state.locations || [];
  ensureLocationNodes();
}

function ensureLocationNodes() {
  for (const loc of locations) {
    if (document.getElementById("loc-" + loc.index)) continue;
    const node = document.createElement("div");
    node.id = "loc-" + loc.index;
    node.className = "loc-node";
    node.style.left = loc.x + "%";
    node.style.top = loc.y + "%";
    node.innerHTML = `<div class="loc-emoji">${loc.emoji}</div><div class="loc-name">${loc.name}</div>`;
    el.map.appendChild(node);
  }
}

function renderMap(state) {
  for (const agent of state.agents) {
    const loc = locations[agent.location] || locations[0];
    if (!loc) continue;
    let node = agentEls.get(agent.id);
    if (!node) {
      node = document.createElement("div");
      node.className = "agent";
      node.dataset.id = agent.id;
      node.innerHTML = `
        <div class="pulse"></div>
        <div class="dot">${agent.emoji}</div>
        <div class="tag">${agent.name}</div>`;
      node.addEventListener("click", () => selectAgent(agent.id));
      node.addEventListener("mouseenter", (e) => showTooltip(e, agent));
      node.addEventListener("mousemove", (e) => moveTooltip(e));
      node.addEventListener("mouseleave", hideTooltip);
      el.map.appendChild(node);
      agentEls.set(agent.id, node);
    }
    node.style.left = loc.x + "%";
    node.style.top = loc.y + "%";
    node.classList.toggle("selected", agent.id === selectedId);
    node.querySelector(".tag").textContent =
      agent.name + " · " + agent.current_action;
  }
}

function showTooltip(e, agent) {
  el.tooltip.innerHTML = `
    <div class="t-name">${agent.emoji} ${agent.name}</div>
    <div class="t-action">${agent.location_emoji} ${agent.location_name} — ${agent.current_action}</div>`;
  el.tooltip.classList.remove("hidden");
  moveTooltip(e);
}

function moveTooltip(e) {
  const rect = el.map.getBoundingClientRect();
  el.tooltip.style.left = Math.min(e.clientX - rect.left + 14, rect.width - 280) + "px";
  el.tooltip.style.top = Math.min(e.clientY - rect.top + 14, rect.height - 60) + "px";
}

function hideTooltip() {
  el.tooltip.classList.add("hidden");
}

function selectAgent(id) {
  selectedId = id;
  for (const [aid, node] of agentEls) {
    node.classList.toggle("selected", aid === id);
  }
  if (!lastState) return;
  const agent = lastState.agents.find((a) => a.id === id);
  if (agent) renderDetail(agent);
}

let lastState = null;

function renderDetail(agent) {
  el.detail.innerHTML = `
    <div class="d-name"><span class="e">${agent.emoji}</span>${agent.name}</div>
    <div class="d-bio">${agent.bio}</div>
    <div class="d-row"><span class="k">Now</span><span>${agent.location_emoji} ${agent.location_name} — ${agent.current_action}</span></div>
    <div class="d-row"><span class="k">Traits</span><span>${(agent.traits||"").split(", ").map(t => `<span class="chip">${t}</span>`).join("")}</span></div>
    <div class="d-section">
      <h3>Today's plan</h3>
      <ul class="d-list">${agent.plan.map(p => `<li>${p}</li>`).join("") || "<li>—</li>"}</ul>
    </div>
    <div class="d-section">
      <h3>Relationships</h3>
      <ul class="d-list">${agent.relationships.map(r => `<li class="rel"><span>${r[0]}</span><span>♥ ${r[1]}</span></li>`).join("") || "<li>—</li>"}</ul>
    </div>
    <div class="d-section">
      <h3>Recent memories</h3>
      <ul class="d-list">${agent.recent_memories.map(m => `<li>${m}</li>`).join("") || "<li>—</li>"}</ul>
    </div>`;
}

function appendEvents(events) {
  const frag = document.createDocumentFragment();
  for (const ev of events) {
    const li = document.createElement("li");
    li.className = "k-" + ev.kind;
    const t = document.createElement("span");
    t.className = "t";
    t.textContent = `[${ev.day}.${ev.time}]`;
    li.appendChild(t);
    li.appendChild(document.createTextNode(ev.text));
    frag.appendChild(li);
  }
  const stick = el.feed.scrollTop + el.feed.clientHeight >= el.feed.scrollHeight - 30;
  el.feed.appendChild(frag);
  while (el.feed.childElementCount > 300) el.feed.firstChild.remove();
  if (stick) el.feed.scrollTop = el.feed.scrollHeight;
}

async function tick() {
  await poll();
  setTimeout(tick, POLL_MS);
}

fetch("/api/state")
  .then((r) => r.json())
  .then((s) => {
    lastState = s;
    renderClock(s);
    renderMap(s);
  });

tick();

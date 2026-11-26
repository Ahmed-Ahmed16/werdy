// Offline smoke test for the built web app (#180).
//
// Serves the built files on localhost and opens them in headless Chrome.
// Then the server stops answering (a weak signal: requests hang) and the app
// must still open and save a mark. Then the server stops entirely (airplane
// mode: requests fail at once) and the mark must still be there. Every host except localhost is unreachable for the
// whole run, so Google Fonts never load, as on a phone with no signal.
//
// Run after `dx build --platform web --release --base-path werdy` and
// `scripts/prepare-pages.ps1`:
//   node scripts/tests/offline-smoke.mjs target/dx/werdy-app/release/web/public
// CHROME=<path> picks the browser; otherwise common install paths are tried.
// Chrome only: WebKit is not installed here or in CI, so iOS still needs a
// check on a real iPhone.
//
// The run can never hang: Chrome and the server are stopped on every path,
// and a watchdog ends the process after RUN_LIMIT_MS whatever is still open.

import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { extname, join, normalize, resolve } from "node:path";

const publicDir = resolve(process.argv[2] ?? "target/dx/werdy-app/release/web/public");
const BASE = "/werdy/";
const TYPES = {
  ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css",
  ".wasm": "application/wasm", ".json": "application/json", ".webmanifest": "application/manifest+json",
  ".png": "image/png", ".svg": "image/svg+xml",
};

// A full run takes well under a minute; anything near these limits is a hang.
const RUN_LIMIT_MS = 5 * 60 * 1000;
const CHROME_START_MS = 60 * 1000;
const CONNECT_MS = 10 * 1000;

function fail(message) {
  console.error(`offline smoke test failed: ${message}`);
  process.exitCode = 1;
  throw new Error(message);
}

// Everything that could keep node alive, so every exit path can stop it.
const running = { chrome: undefined, server: undefined, socket: undefined };

function stopEverything() {
  running.socket?.close();
  const chrome = running.chrome;
  if (chrome && chrome.exitCode === null && chrome.signalCode === null) {
    try {
      // On Linux the Chrome wrapper script and its helpers share a process
      // group; killing the group stops them all, not just the wrapper.
      if (process.platform === "win32") chrome.kill();
      else process.kill(-chrome.pid, "SIGKILL");
    } catch {
      chrome.kill("SIGKILL");
    }
  }
  chrome?.stderr?.destroy();
  running.server?.closeAllConnections();
  running.server?.close();
}

const watchdog = setTimeout(() => {
  console.error(`offline smoke test failed: still running after ${RUN_LIMIT_MS / 1000} s`);
  stopEverything();
  process.exit(1);
}, RUN_LIMIT_MS);
watchdog.unref();

const network = { hanging: false };

function serve() {
  const server = createServer((request, response) => {
    if (network.hanging) return;
    const path = new URL(request.url, "http://localhost").pathname;
    if (!path.startsWith(BASE)) {
      response.writeHead(404).end();
      return;
    }
    let file = normalize(join(publicDir, decodeURIComponent(path.slice(BASE.length))));
    if (!file.startsWith(publicDir)) {
      response.writeHead(403).end();
      return;
    }
    if (path.endsWith("/")) file = join(file, "index.html");
    if (!existsSync(file)) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    response.end(readFileSync(file));
  });
  return new Promise((done) => server.listen(0, "127.0.0.1", () => done(server)));
}

function chromePath() {
  const candidates = [
    process.env.CHROME,
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  ];
  return candidates.find((path) => path && existsSync(path)) ?? fail("no Chrome found; set CHROME");
}

async function launch() {
  const profile = mkdtempSync(join(tmpdir(), "werdy-offline-"));
  const chrome = spawn(chromePath(), [
    "--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
    "--no-first-run", "--no-default-browser-check", "--disable-gpu",
    "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost",
  ], { stdio: ["ignore", "ignore", "pipe"], detached: process.platform !== "win32" });
  running.chrome = chrome;
  // Keep the end of Chrome's log to explain a failed start.
  let log = "";
  chrome.stderr.on("data", (chunk) => {
    log = (log + chunk).slice(-4000);
  });
  chrome.on("error", () => {});
  const startFailed = (why) => fail(`Chrome ${why}. Its last output:\n${log || "(none)"}`);
  const portFile = join(profile, "DevToolsActivePort");
  const deadline = Date.now() + CHROME_START_MS;
  while (!existsSync(portFile) || !readFileSync(portFile, "utf8").includes("\n")) {
    if (chrome.exitCode !== null || chrome.signalCode !== null) startFailed(`exited (${chrome.exitCode ?? chrome.signalCode}) before opening DevTools`);
    if (Date.now() > deadline) startFailed(`did not open DevTools within ${CHROME_START_MS / 1000} s`);
    await wait(100);
  }
  const port = readFileSync(portFile, "utf8").split("\n")[0];
  const list = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(CONNECT_MS) });
  const targets = await list.json();
  const page = targets.find((target) => target.type === "page") ?? fail("no page target");
  return connect(page.webSocketDebuggerUrl);
}

function connect(url) {
  const socket = new WebSocket(url);
  running.socket = socket;
  const pending = new Map();
  let next = 0;
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    pending.get(message.id)?.(message);
    pending.delete(message.id);
  });
  // A page stuck in a navigation can leave a call unanswered.
  const send = (method, params = {}) => new Promise((done) => {
    const id = ++next;
    const timer = setTimeout(() => {
      pending.delete(id);
      done({ timedOut: true });
    }, 3000);
    pending.set(id, (reply) => {
      clearTimeout(timer);
      done(reply);
    });
    socket.send(JSON.stringify({ id, method, params }));
  });
  return new Promise((done, failed) => {
    const timer = setTimeout(() => failed(new Error("DevTools socket did not open")), CONNECT_MS);
    socket.addEventListener("open", () => {
      clearTimeout(timer);
      done({ socket, send });
    });
    socket.addEventListener("error", () => {
      clearTimeout(timer);
      failed(new Error("DevTools socket failed to open"));
    });
  });
}

const wait = (milliseconds) => new Promise((done) => setTimeout(done, milliseconds));

async function evaluate(cdp, expression) {
  const reply = await cdp.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  return reply.result?.result?.value;
}

async function until(cdp, expression, what, seconds = 60) {
  const deadline = Date.now() + seconds * 1000;
  while (Date.now() < deadline) {
    const value = await evaluate(cdp, expression).catch(() => undefined);
    if (value) return value;
    await wait(200);
  }
  fail(`timed out after ${seconds} s waiting for ${what}`);
}

const RENDERED = `document.querySelectorAll('#main button').length > 5`;
async function reloadAndRender(cdp, what, seconds) {
  const marker = randomUUID();
  if (await evaluate(cdp, `window.__werdySmokeOld = ${JSON.stringify(marker)}`) !== marker) {
    fail(`could not mark the old page before waiting for ${what}`);
  }
  const reply = await cdp.send("Page.reload");
  if (reply.timedOut) fail(`Page.reload timed out before waiting for ${what}`);
  if (reply.id === undefined || !reply.result) {
    fail(`Page.reload failed before waiting for ${what}: ${reply.error?.message ?? "missing reply"}`);
  }
  // The old page can still answer while the worker waits for the network.
  await until(cdp, `typeof window.__werdySmokeOld === 'undefined' && (${RENDERED})`, what, seconds);
}

const CONTROLLED = `!!navigator.serviceWorker.controller`;
const RECORDS = `localStorage.getItem('werdy.demo.records')`;
// Opens the first group if every group is folded, then taps the first open mark.
const TAP = `(() => {
  let mark = document.querySelector('button[data-kind="done"][aria-pressed="false"]');
  if (!mark) {
    [...document.querySelectorAll('button[aria-expanded="false"]')].forEach((b) => b.click());
    return false;
  }
  mark.click();
  return true;
})()`;

async function main() {
  if (!existsSync(join(publicDir, "service-worker.js"))) {
    fail(`${publicDir} has no service-worker.js; run scripts/prepare-pages.ps1 first`);
  }
  try {
    const server = await serve();
    running.server = server;
    const address = `http://localhost:${server.address().port}${BASE}?demo`;
    const cdp = await launch();
    await cdp.send("Page.enable");
    await cdp.send("Page.navigate", { url: address });
    await until(cdp, RENDERED, "the first online render");
    await until(cdp, `navigator.serviceWorker.ready.then((r) => r.active?.state === 'activated')`, "the worker to activate");
    // The page that installed the worker is not controlled by it; the next one is.
    await reloadAndRender(cdp, "the second online render");
    await until(cdp, CONTROLLED, "the worker to control the page");

    network.hanging = true;
    console.log("weak signal: the server takes requests but never answers");
    await reloadAndRender(cdp, "the app to render while the network hangs", 20);
    if (!(await evaluate(cdp, CONTROLLED))) fail("the offline page is not served by the worker");
    const before = await evaluate(cdp, RECORDS);
    await until(cdp, TAP, "a mark button to tap");
    const saved = await until(cdp, `(${RECORDS} !== ${JSON.stringify(before)}) && ${RECORDS}`, "the mark to be saved");

    const closed = new Promise((done) => server.close(done));
    server.closeAllConnections();
    await closed;
    console.log("airplane mode: the server is gone");
    await reloadAndRender(cdp, "the app to render offline");
    if ((await evaluate(cdp, RECORDS)) !== saved) fail("the mark tapped offline was lost on reload");
    console.log("ok: the app opens on a hanging network and offline, and a mark tapped offline is kept");
  } finally {
    stopEverything();
  }
}

main().catch((error) => {
  if (!process.exitCode) {
    console.error(error);
    process.exitCode = 1;
  }
});

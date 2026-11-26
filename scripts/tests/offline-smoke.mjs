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

import { createServer } from "node:http";
import { spawn } from "node:child_process";
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

function fail(message) {
  console.error(`offline smoke test failed: ${message}`);
  process.exitCode = 1;
  throw new Error(message);
}

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
  ], { stdio: "ignore" });
  const portFile = join(profile, "DevToolsActivePort");
  for (let tries = 0; !existsSync(portFile) || !readFileSync(portFile, "utf8").includes("\n"); tries++) {
    if (tries > 100) fail("Chrome did not start");
    await wait(100);
  }
  const port = readFileSync(portFile, "utf8").split("\n")[0];
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = targets.find((target) => target.type === "page") ?? fail("no page target");
  return { chrome, socket: await connect(page.webSocketDebuggerUrl) };
}

function connect(url) {
  const socket = new WebSocket(url);
  const pending = new Map();
  let next = 0;
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    pending.get(message.id)?.(message);
    pending.delete(message.id);
  });
  // A page stuck in a navigation can leave a call unanswered, so each call
  // gets a time limit and then resolves empty, which callers read as "not yet".
  const send = (method, params = {}) => new Promise((done) => {
    const id = ++next;
    pending.set(id, done);
    socket.send(JSON.stringify({ id, method, params }));
    setTimeout(() => done({}), 3000);
  });
  return new Promise((done) => socket.addEventListener("open", () => done({ socket, send })));
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
  const server = await serve();
  const address = `http://localhost:${server.address().port}${BASE}?demo`;
  const { chrome, socket: cdp } = await launch();
  try {
    await cdp.send("Page.enable");
    await cdp.send("Page.navigate", { url: address });
    await until(cdp, RENDERED, "the first online render");
    await until(cdp, `navigator.serviceWorker.ready.then((r) => r.active?.state === 'activated')`, "the worker to activate");
    // The page that installed the worker is not controlled by it; the next one is.
    await cdp.send("Page.reload");
    await until(cdp, CONTROLLED, "the worker to control the page");
    await until(cdp, RENDERED, "the second online render");

    network.hanging = true;
    console.log("weak signal: the server takes requests but never answers");
    await cdp.send("Page.reload");
    await until(cdp, RENDERED, "the app to render while the network hangs", 20);
    if (!(await evaluate(cdp, CONTROLLED))) fail("the offline page is not served by the worker");
    const before = await evaluate(cdp, RECORDS);
    await until(cdp, TAP, "a mark button to tap");
    const saved = await until(cdp, `(${RECORDS} !== ${JSON.stringify(before)}) && ${RECORDS}`, "the mark to be saved");

    const closed = new Promise((done) => server.close(done));
    server.closeAllConnections();
    await closed;
    console.log("airplane mode: the server is gone");
    await cdp.send("Page.reload");
    await until(cdp, RENDERED, "the app to render offline");
    if ((await evaluate(cdp, RECORDS)) !== saved) fail("the mark tapped offline was lost on reload");
    console.log("ok: the app opens on a hanging network and offline, and a mark tapped offline is kept");
  } finally {
    cdp.socket.close();
    chrome.kill();
    server.closeAllConnections();
    server.close();
  }
}

main().catch((error) => {
  if (!process.exitCode) {
    console.error(error);
    process.exitCode = 1;
  }
});

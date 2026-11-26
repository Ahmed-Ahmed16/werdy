// Runs the service worker template against fake caches and a fake network.
// Run: node --test --test-timeout=5000 scripts/tests/service-worker.test.mjs
// SERVICE_WORKER=<file> runs the same checks against another copy.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const templatePath = process.env.SERVICE_WORKER ??
  fileURLToPath(new URL("../service-worker.template.js", import.meta.url));
const ORIGIN = "https://example.test";
const FONT_CSS = "https://fonts.googleapis.com/css2?family=Plex";

function fakeCaches(stores = new Map(), { rejectPut = false } = {}) {
  const key = (request) => new URL(typeof request === "string" ? request : request.url, ORIGIN).href;
  const open = async (name) => {
    if (!stores.has(name)) stores.set(name, new Map());
    const entries = stores.get(name);
    return {
      match: async (request) => entries.get(key(request))?.clone(),
      put: async (request, response) => {
        if (rejectPut) throw new Error("cache write failed");
        entries.set(key(request), response);
      },
      addAll: async (urls) => {
        for (const url of urls) entries.set(key(url), new Response(`cached ${url}`));
      },
    };
  };
  return {
    stores,
    open,
    keys: async () => [...stores.keys()],
    delete: async (name) => stores.delete(name),
    match: async (request) => {
      for (const name of stores.keys()) {
        const found = await (await open(name)).match(request);
        if (found) return found;
      }
      return undefined;
    },
  };
}

function loadWorker(version, stores, cacheOptions) {
  const handlers = {};
  const network = { mode: "online" };
  const caches = fakeCaches(stores, cacheOptions);
  const fetch = (request) => {
    if (network.mode === "offline") return Promise.reject(new TypeError("Load failed"));
    if (network.mode === "hanging") return new Promise(() => {});
    return Promise.resolve(new Response(`network ${request.url ?? request}`));
  };
  const self = {
    location: new URL(ORIGIN),
    addEventListener: (type, handler) => { handlers[type] = handler; },
  };
  const source = readFileSync(templatePath, "utf8")
    .replace("__VERSION__", version)
    .replace("__PRECACHE__", JSON.stringify(["/werdy/index.html", "/werdy/assets/app.wasm"]))
    .replace(/NETWORK_TIMEOUT_MS = \d+/, "NETWORK_TIMEOUT_MS = 20");
  vm.runInNewContext(source, {
    self, caches, fetch, URL, Response, Promise, Error, TypeError, setTimeout, clearTimeout,
    clients: {},
  });
  return { handlers, network, caches };
}

async function dispatch(worker, type, extra = {}) {
  let waited;
  let responded;
  worker.handlers[type]({
    ...extra,
    waitUntil: (promise) => { waited = promise; },
    respondWith: (promise) => { responded = promise; },
  });
  await waited;
  return responded;
}

async function installed(version = "one", stores = undefined, cacheOptions) {
  const worker = loadWorker(version, stores, cacheOptions);
  await dispatch(worker, "install");
  await dispatch(worker, "activate");
  return worker;
}

// Resolves to the response text, or to "timeout" if nothing answers in time.
async function body(worker, url, mode = "cors") {
  const request = { url: new URL(url, ORIGIN).href, method: "GET", mode };
  const response = await dispatch(worker, "fetch", { request });
  assert.ok(response, `the worker answers ${url}`);
  const late = new Promise((resolve) => setTimeout(() => resolve("timeout"), 500));
  return Promise.race([Promise.resolve(response).then((r) => r.text()), late]);
}

test("offline, opening the app serves the cached app shell", async () => {
  const worker = await installed();
  worker.network.mode = "offline";
  assert.equal(await body(worker, "/werdy/", "navigate"), "cached /werdy/index.html");
});

test("a network that never answers still opens the cached app shell", async () => {
  const worker = await installed();
  worker.network.mode = "hanging";
  assert.equal(await body(worker, "/werdy/", "navigate"), "cached /werdy/index.html");
});

test("online, opening the app loads the newest page", async () => {
  const worker = await installed();
  assert.equal(await body(worker, "/werdy/", "navigate"), `network ${ORIGIN}/werdy/`);
});

test("the font stylesheet seen online is served offline", async () => {
  const worker = await installed();
  assert.equal(await body(worker, FONT_CSS), `network ${FONT_CSS}`);
  worker.network.mode = "offline";
  assert.equal(await body(worker, FONT_CSS), `network ${FONT_CSS}`);
});

test("a failed cache write still serves the font from the network on a cache miss", async () => {
  const worker = await installed("one", undefined, { rejectPut: true });
  assert.equal(await body(worker, FONT_CSS), `network ${FONT_CSS}`);
});

test("cached fonts survive an app update and the old app cache goes", async () => {
  const stores = new Map();
  const first = await installed("one", stores);
  await body(first, FONT_CSS);
  const next = await installed("two", stores);
  next.network.mode = "offline";
  assert.equal(await body(next, FONT_CSS), `network ${FONT_CSS}`);
  assert.ok(!stores.has("werdy-one"));
});

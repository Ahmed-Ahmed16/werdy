import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(new URL("../src/platform/push.js", import.meta.url), "utf8");

function browser(registration, { ready = new Promise(() => {}), supported = true, getRegistration } = {}) {
    let expire;
    const serviceWorker = {
        ready,
        getRegistration: getRegistration ?? (async scope => {
            assert.equal(scope, "/werdy/");
            return registration;
        })
    };
    const context = vm.createContext({
        navigator: { serviceWorker, userAgent: "", maxTouchPoints: 0 },
        window: supported ? { PushManager: class {}, Notification: class {} } : {},
        Notification: { permission: "granted", requestPermission: async () => "granted" },
        matchMedia: () => ({ matches: false }),
        atob,
        setTimeout(callback, delay) { assert.equal(delay, 10_000); expire = callback; return 1; },
        clearTimeout() {}
    });
    return {
        run: operation => vm.runInContext(`(async () => {const operation=${JSON.stringify(operation)}; const key="AQ"; ${script}\n})()`, context),
        expire: () => { assert.ok(expire, "operation must set timeout"); expire(); }
    };
}

test("missing service worker lets subscription lookup and removal finish", async () => {
    const page = browser(undefined);
    const answer = operation => Promise.race([
        page.run(operation),
        new Promise(resolve => setTimeout(() => resolve("hung"), 50))
    ]);
    assert.equal(await answer("current"), null);
    assert.equal(await answer("unsubscribe"), true);
});

test("unsupported browser treats removal as complete", async () => {
    assert.equal(await browser(undefined, { supported: false }).run("unsubscribe"), true);
});

test("registered subscription can be read and removed", async () => {
    let removed = false;
    const subscription = {
        toJSON: () => ({ endpoint: "https://push.example.test/a", keys: { p256dh: "key", auth: "auth" } }),
        unsubscribe: async () => { removed = true; return true; }
    };
    const page = browser({ pushManager: { getSubscription: async () => subscription } });
    assert.equal((await page.run("current")).endpoint, "https://push.example.test/a");
    assert.equal(await page.run("unsubscribe"), true);
    assert.equal(removed, true);
});

test("registration lookup timeout answers Rust", async () => {
    const page = browser(undefined, { getRegistration: () => new Promise(() => {}) });
    const pending = page.run("current");
    page.expire();
    await assert.rejects(pending, /timed out/);
});

test("subscribe timeout answers Rust when registration never becomes ready", async () => {
    const page = browser(undefined);
    const pending = page.run("subscribe");
    await Promise.resolve();
    page.expire();
    await assert.rejects(pending, /timed out/);
});

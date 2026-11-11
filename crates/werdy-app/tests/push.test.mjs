import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(new URL("../src/platform/push.js", import.meta.url), "utf8");

function browser(registration, { ready = new Promise(() => {}), supported = true, getRegistration, requestPermission = async () => "granted" } = {}) {
    let expire;
    let clock = 0;
    let deadline;
    let startTimer;
    const timerStarted = new Promise(resolve => { startTimer = resolve; });
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
        Notification: { permission: "granted", requestPermission },
        matchMedia: () => ({ matches: false }),
        atob,
        setTimeout(callback, delay) { assert.equal(delay, 10_000); expire = callback; deadline = clock + delay; startTimer(); return 1; },
        clearTimeout() { expire = undefined; }
    });
    return {
        run: operation => vm.runInContext(`(async () => {const operation=${JSON.stringify(operation)}; const key="AQ"; ${script}\n})()`, context),
        timerStarted,
        expire: () => { assert.ok(expire, "operation must set timeout"); expire(); },
        advance: milliseconds => { clock += milliseconds; if (expire && clock >= deadline) expire(); }
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
    await page.timerStarted;
    page.expire();
    await assert.rejects(pending, /timed out/);
});

test("permission answered after 15 seconds still subscribes", async () => {
    let answerPermission;
    const prompt = new Promise(resolve => { answerPermission = resolve; });
    const subscription = {
        toJSON: () => ({ endpoint: "https://push.example.test/late", keys: { p256dh: "key", auth: "auth" } })
    };
    const registration = { pushManager: { subscribe: async () => subscription } };
    const page = browser(registration, { ready: Promise.resolve(registration), requestPermission: () => prompt });
    const pending = page.run("subscribe");
    page.advance(15_000);
    answerPermission("granted");
    assert.equal((await pending).endpoint, "https://push.example.test/late");
});

test("subscribe timeout removes a subscription created by the browser", async () => {
    let removed = false;
    const subscription = { unsubscribe: async () => { removed = true; return true; } };
    const registration = { pushManager: {
        subscribe: () => new Promise(() => {}),
        getSubscription: async () => subscription
    } };
    const page = browser(registration, { ready: Promise.resolve(registration) });
    const pending = page.run("subscribe");
    await page.timerStarted;
    await Promise.resolve();
    page.expire();
    await assert.rejects(pending, /timed out/);
    assert.equal(removed, true);
});

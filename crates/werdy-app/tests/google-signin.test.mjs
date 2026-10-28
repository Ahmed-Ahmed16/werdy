import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(new URL("../src/account/signin/google.js", import.meta.url), "utf8");

function browser(reply) {
    const messages = [];
    let options, stop, expire;
    let loads = 0;
    const container = {closest: () => ({lang: "ar"})};
    const google = {accounts: {id: {
        initialize(settings) {options = settings;},
        renderButton(target, style) {
            assert.equal(target, container);
            assert.equal(style.locale, "ar");
            if (reply !== undefined) queueMicrotask(() => options.callback(reply));
        }
    }}};
    const context = vm.createContext({
        dioxus: {send(message) {messages.push(JSON.parse(JSON.stringify(message)));}, recv() {return new Promise(resolve => {stop = resolve;});}},
        setTimeout(callback) {expire = callback; return 1;}, clearTimeout() {},
        document: {
            getElementById() {return container;}, createElement() {return {};},
            head: {appendChild(element) {
                loads++;
                assert.equal(element.src, "https://accounts.google.com/gsi/client");
                context.google = google;
                queueMicrotask(() => element.onload());
            }}
        }
    });
    return {context, messages, options: () => options, stop: () => stop(), expire: () => expire(), loads: () => loads};
}

function run(page) {
    return vm.runInContext(`(async () => {const clientId="client"; const nonce="hashed_nonce"; ${script}\n})()`, page.context);
}

test("Google button returns credential and reuses loaded script", async () => {
    const page = browser({credential: "token"});
    await run(page);
    assert.equal(page.options().nonce, "hashed_nonce");
    assert.deepEqual(page.messages, [{state: "ok", idToken: "token"}]);
    await run(page);
    assert.equal(page.loads(), 1);
});

test("invalid replies and load failures expose no secret", async () => {
    for (const reply of [null, {}, {credential: ""}, {credential: {secret: "token"}}]) {
        const page = browser(reply);
        await run(page);
        assert.deepEqual(page.messages, [{state: "error"}]);
    }
    const page = browser(undefined);
    page.context.document.head.appendChild = element => queueMicrotask(() => element.onerror());
    await run(page);
    assert.deepEqual(page.messages, [{state: "error"}]);
});

test("cancelled component ignores late callback and timeout reports error", async () => {
    const page = browser(undefined);
    const pending = run(page);
    await new Promise(resolve => setImmediate(resolve));
    page.stop();
    await pending;
    page.options().callback({credential: "late_token"});
    assert.deepEqual(page.messages, []);
    const timed = browser(undefined);
    const timeout = run(timed);
    timed.expire();
    await timeout;
    assert.deepEqual(timed.messages, [{state: "error"}]);
});

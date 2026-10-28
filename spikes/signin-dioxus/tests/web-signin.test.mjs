import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const source = readFileSync(new URL("../src/web_signin.rs", import.meta.url), "utf8");
const script = source.match(/const GOOGLE_SCRIPT: &str = r#"([\s\S]*?)"#;/)?.[1];
assert.ok(script, "Google script must exist");

function browser(credential) {
    const messages = [];
    const container = {};
    let options;
    let loads = 0;
    const google = { accounts: { id: {
        initialize(settings) { options = settings; },
        renderButton(target, style) {
            assert.equal(target, container);
            assert.equal(style.theme, "outline");
            assert.equal(style.size, "large");
            queueMicrotask(() => options.callback(credential));
        },
    } } };
    const context = vm.createContext({
        dioxus: { send(message) { messages.push(message); } },
        document: {
            createElement() { return {}; },
            getElementById(id) { return id === "gsi-button" ? container : null; },
            head: { appendChild(element) {
                loads += 1;
                assert.equal(element.src, "https://accounts.google.com/gsi/client");
                context.google = google;
                queueMicrotask(() => element.onload());
            } },
        },
    });
    return { context, messages, options: () => options, loads: () => loads };
}

function run(context, clientId = "client", nonce = "nonce") {
    const bindings = `const clientId = ${JSON.stringify(clientId)}; const nonce = ${JSON.stringify(nonce)};`;
    return vm.runInContext(`(async () => { ${bindings}\n${script}\n })()`, context);
}

test("GIS callback returns credential and reuses loaded script", async () => {
    const page = browser({ credential: "google.token.signature" });
    const quotedClient = 'client"; throw new Error("injected"); //';
    await run(page.context, quotedClient, "hash\nwith\"quotes");
    assert.equal(page.options().client_id, quotedClient);
    assert.equal(page.options().nonce, "hash\nwith\"quotes");
    assert.deepEqual(page.messages, ["google.token.signature"]);
    await run(page.context);
    assert.equal(page.loads(), 1);
    assert.equal(page.messages.length, 2);
});

test("invalid GIS credentials become errors without token contents", async () => {
    for (const credential of [null, {}, { credential: "" }, { credential: { secret: "token" } }]) {
        const page = browser(credential);
        await run(page.context);
        assert.deepEqual(page.messages, ["GIS error: Google returned no credential"]);
    }
});

test("script load failure reaches Rust instead of waiting for callback", async () => {
    const page = browser({ credential: "unused" });
    page.context.document.head.appendChild = (element) => queueMicrotask(() => element.onerror());
    await run(page.context);
    assert.deepEqual(page.messages, ["GIS error: Could not load or initialize Google sign in"]);
});

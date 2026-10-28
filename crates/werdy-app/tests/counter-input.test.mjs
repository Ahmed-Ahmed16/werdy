import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(new URL("../src/ui/counter-input.js", import.meta.url), "utf8");

test("Arabic digits from typing and paste reach the number input as ASCII", () => {
    for (const source of ["beforeinput", "paste"]) {
        const listeners = {};
        const insertions = [];
        const input = { focus() {}, select() {}, addEventListener(kind, listener) { listeners[kind] = listener; } };
        vm.runInNewContext(script, {
            inputId: "count-input-1",
            document: {
                getElementById: () => input,
                execCommand(command, showUi, text) { insertions.push([command, showUi, text]); return true; }
            }
        });
        for (const [text, expected] of [["٠١٢٣٤٥٦٧٨٩", "0123456789"], ["١0٢", "102"], ["-١", "-1"], ["١.٥", "1.5"]]) {
            let prevented = false;
            listeners[source]({ data: text, clipboardData: { getData: () => text }, preventDefault() { prevented = true; } });
            assert.equal(prevented, true);
            assert.deepEqual(insertions.at(-1), ["insertText", false, expected]);
        }
        const before = insertions.length;
        listeners[source]({ data: "12", clipboardData: { getData: () => "12" }, preventDefault() { assert.fail("ASCII uses browser input"); } });
        assert.equal(insertions.length, before);
    }
});

test("Arabic digits still reach the input when insertText is unavailable", () => {
    const listeners = {};
    const events = [];
    const input = {
        value: "7",
        focus() {},
        select() {},
        addEventListener(kind, listener) { listeners[kind] = listener; },
        dispatchEvent(event) { events.push(event.type); return true; }
    };
    vm.runInNewContext(script, {
        inputId: "count-input-1",
        Event: class { constructor(type, options) { this.type = type; this.bubbles = options?.bubbles; } },
        document: { getElementById: () => input, execCommand: () => false }
    });
    const typed = text => listeners.beforeinput({ data: text, preventDefault() {} });
    // The current value starts selected, so the first digit replaces it.
    typed("١");
    assert.equal(input.value, "1");
    typed("٢");
    assert.equal(input.value, "12");
    assert.deepEqual(events, ["input", "input"]);
});

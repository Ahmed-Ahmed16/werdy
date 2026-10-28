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

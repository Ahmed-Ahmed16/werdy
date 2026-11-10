import assert from "node:assert/strict";
import test from "node:test";
import { buildHeaders, buildPayload, classifyReply } from "./core.ts";

test("builds a Declarative Web Push payload with the topic", () => {
  const payload = JSON.parse(buildPayload("Dhuhr deeds", "Open today's sheet", "abc_123"));
  assert.deepEqual(payload, {
    web_push: 8030,
    notification: {
      title: "Dhuhr deeds",
      body: "Open today's sheet",
      navigate: "https://ahmed-ahmed16.github.io/werdy/",
      tag: "abc_123",
    },
  });
});

test("builds delivery headers with the topic unchanged", () => {
  assert.deepEqual(buildHeaders("AbC_09-xy"), {
    TTL: "900",
    Urgency: "high",
    Topic: "AbC_09-xy",
  });
});

test("classifies push service replies", () => {
  for (const status of [200, 201, 202, 204]) {
    assert.equal(classifyReply(status), "sent");
  }
  for (const status of [404, 410]) {
    assert.equal(classifyReply(status), "drop_subscription");
  }
  for (const status of [301, 413, 429, 500]) {
    assert.equal(classifyReply(status), "log_and_drop_row");
  }
});

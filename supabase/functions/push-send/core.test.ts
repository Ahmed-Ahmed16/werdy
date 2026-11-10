import assert from "node:assert/strict";
import test from "node:test";
import { buildHeaders, buildPayload, classifyReply, runLimited, shouldClaimMore } from "./core.ts";

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

test("processes every row with at most the allowed sends in flight after a failure", async () => {
  let active = 0;
  let peak = 0;
  const processed: number[] = [];
  const outcomes = await runLimited([0, 1, 2, 3, 4, 5, 6], 3, async (item) => {
    active++;
    peak = Math.max(peak, active);
    await new Promise<void>((resolve) => setImmediate(resolve));
    processed.push(item);
    active--;
    if (item === 1) throw new Error("push failed");
    return item * 2;
  });

  assert.equal(peak, 3);
  assert.deepEqual(processed.sort((a, b) => a - b), [0, 1, 2, 3, 4, 5, 6]);
  assert.deepEqual(outcomes.map((outcome) => outcome.status), [
    "fulfilled", "rejected", "fulfilled", "fulfilled", "fulfilled", "fulfilled", "fulfilled",
  ]);
  assert.equal(outcomes[1].status === "rejected" && outcomes[1].reason.message, "push failed");
});

test("claims again only after a nonempty batch within the time budget", () => {
  assert.equal(shouldClaimMore(1_000, 1_000, 20), true);
  assert.equal(shouldClaimMore(1_000, 100_999, 1), true);
  assert.equal(shouldClaimMore(1_000, 101_000, 1), false);
  assert.equal(shouldClaimMore(1_000, 2_000, 0), false);
});

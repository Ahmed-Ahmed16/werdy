import assert from "node:assert/strict";
import test from "node:test";
import { buildHeaders, buildPayload, classifyReply, isAllowedPushEndpoint, planSends, runLimited, shouldClaimMore } from "./core.ts";

test("allows only push service endpoints with safe URL parts", () => {
  for (const endpoint of [
    "https://fcm.googleapis.com/send",
    "HTTPS://FCM.GOOGLEAPIS.COM:443/send",
    "https://updates.push.services.mozilla.com/send",
    "https://web.push.apple.com/send",
    "https://a.b.push.apple.com/send",
    "https://a.notify.windows.com/send",
    "https://a.b.notify.windows.com/send",
  ]) {
    assert.equal(isAllowedPushEndpoint(endpoint), true, endpoint);
  }
  for (const endpoint of [
    "not a URL",
    "http://fcm.googleapis.com/send",
    "https://user@fcm.googleapis.com/send",
    "https://@fcm.googleapis.com/send",
    "https://user:pass@fcm.googleapis.com/send",
    "https://fcm.googleapis.com:8443/send",
    "https://evil.test/send",
    "https://push.apple.com.evil.test/send",
    "https://evilnotify.windows.com/send",
    "https://push.apple.com/send",
    "https://notify.windows.com/send",
  ]) {
    assert.equal(isAllowedPushEndpoint(endpoint), false, endpoint);
  }
});

test("disallowed rows never reach the send function", async () => {
  const rows = [
    { subscription: "safe", endpoint: "https://fcm.googleapis.com/send" },
    { subscription: "unsafe", endpoint: "https://evil.test/send" },
  ];
  const planned = planSends(rows);
  const reached: string[] = [];
  await runLimited(planned.send, 2, async (row) => { reached.push(row.subscription); });
  assert.deepEqual(reached, ["safe"]);
  assert.deepEqual(planned.drop.map((row) => row.subscription), ["unsafe"]);
});

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

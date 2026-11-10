export type ReplyAction = "sent" | "drop_subscription" | "log_and_drop_row";

export function buildPayload(title: string, body: string, topic: string): string {
  return JSON.stringify({
    web_push: 8030,
    notification: {
      title,
      body,
      navigate: "https://ahmed-ahmed16.github.io/werdy/",
      tag: topic,
    },
  });
}

export function buildHeaders(topic: string): Record<string, string> {
  return { TTL: "900", Urgency: "high", Topic: topic };
}

export function classifyReply(status: number): ReplyAction {
  if (status >= 200 && status < 300) return "sent";
  if (status === 404 || status === 410) return "drop_subscription";
  return "log_and_drop_row";
}

export async function runLimited<T, R>(
  items: readonly T[],
  limit: number,
  worker: (item: T) => Promise<R>,
): Promise<PromiseSettledResult<R>[]> {
  if (!Number.isInteger(limit) || limit < 1) throw new RangeError("limit must be positive");

  const results: PromiseSettledResult<R>[] = new Array(items.length);
  let next = 0;
  async function runNext(): Promise<void> {
    while (next < items.length) {
      const index = next++;
      try {
        results[index] = { status: "fulfilled", value: await worker(items[index]) };
      } catch (reason) {
        results[index] = { status: "rejected", reason };
      }
    }
  }

  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, runNext));
  return results;
}

export function shouldClaimMore(startedMs: number, nowMs: number, lastBatchSize: number): boolean {
  return lastBatchSize > 0 && nowMs - startedMs < 100_000;
}

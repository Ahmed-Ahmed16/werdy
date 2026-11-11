export type ReplyAction = "sent" | "drop_subscription" | "log_and_drop_row";

export function isAllowedPushEndpoint(url: string): boolean {
  try {
    const authority = /^https:\/\/([^/?#]*)/i.exec(url)?.[1];
    if (!authority || /[@\\\s]/.test(authority)) return false;
    const parsed = new URL(url);
    if (parsed.protocol !== "https:" || parsed.username || parsed.password ||
        (parsed.port !== "" && parsed.port !== "443")) return false;
    const host = parsed.hostname.toLowerCase();
    if (host === "fcm.googleapis.com" || host === "updates.push.services.mozilla.com") return true;
    for (const parent of ["push.apple.com", "notify.windows.com"]) {
      if (!host.endsWith(`.${parent}`)) continue;
      const prefix = host.slice(0, -parent.length - 1);
      if (/^[a-z0-9-]+(?:\.[a-z0-9-]+)*$/.test(prefix)) return true;
    }
    return false;
  } catch {
    return false;
  }
}

export function planSends<T extends { endpoint: string }>(rows: readonly T[]): { send: T[]; drop: T[] } {
  const send: T[] = [];
  const drop: T[] = [];
  for (const row of rows) {
    (isAllowedPushEndpoint(row.endpoint) ? send : drop).push(row);
  }
  return { send, drop };
}

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

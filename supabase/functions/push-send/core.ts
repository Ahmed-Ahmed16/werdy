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

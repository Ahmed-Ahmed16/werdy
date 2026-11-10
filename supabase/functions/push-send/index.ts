import webpush from "npm:web-push@3.6.7";
import { buildHeaders, buildPayload, classifyReply } from "./core.ts";

type DueReminder = {
  subscription: string;
  title: string;
  body: string;
  topic: string;
  endpoint: string;
  p256dh: string;
  auth: string;
};

function requiredEnv(name: string): string {
  const value = Deno.env.get(name);
  if (!value) throw new Error(`Missing ${name}`);
  return value;
}

async function rpc<T>(name: string, body: object, url: string, key: string): Promise<T> {
  const response = await fetch(`${url}/rest/v1/rpc/${name}`, {
    method: "POST",
    headers: {
      apikey: key,
      Authorization: `Bearer ${key}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`${name} failed with ${response.status}`);
  if (response.status === 204) return undefined as T;
  return await response.json() as T;
}

Deno.serve(async (request) => {
  if (request.method !== "POST") return new Response("Method not allowed", { status: 405 });

  const cronSecret = Deno.env.get("PUSH_CRON_SECRET");
  if (!cronSecret) return new Response("Push sender failed", { status: 500 });
  if (request.headers.get("Authorization") !== `Bearer ${cronSecret}`) {
    return new Response("Unauthorized", { status: 401 });
  }

  try {
    const url = requiredEnv("SUPABASE_URL");
    const key = requiredEnv("SUPABASE_SERVICE_ROLE_KEY");
    const vapidDetails = {
      subject: requiredEnv("VAPID_SUBJECT"),
      publicKey: requiredEnv("VAPID_PUBLIC_KEY"),
      privateKey: requiredEnv("VAPID_PRIVATE_KEY"),
    };
    const due = await rpc<DueReminder[]>("werdy_push_claim_due", {}, url, key);
    if (!Array.isArray(due)) throw new Error("Claim returned no array");

    let sent = 0;
    let dropped = 0;
    for (const row of due) {
      try {
        const details = webpush.generateRequestDetails(
          { endpoint: row.endpoint, keys: { p256dh: row.p256dh, auth: row.auth } },
          buildPayload(row.title, row.body, row.topic),
          { vapidDetails, TTL: 900, contentEncoding: "aes128gcm", urgency: "high", topic: row.topic },
        );
        const headers = new Headers(details.headers);
        for (const [name, value] of Object.entries(buildHeaders(row.topic))) headers.set(name, value);
        const response = await fetch(details.endpoint, {
          method: "POST",
          headers,
          body: details.body,
          redirect: "error",
        });
        const action = classifyReply(response.status);
        if (action === "sent") {
          sent++;
        } else if (action === "drop_subscription") {
          await rpc<unknown>("werdy_push_drop_subscription", { id: row.subscription }, url, key);
          dropped++;
        } else {
          console.error(`Push failed with ${response.status}`);
        }
      } catch (error) {
        console.error("Push failed", error instanceof Error ? error.message : "unknown error");
      }
    }

    return Response.json({ claimed: due.length, sent, dropped });
  } catch (error) {
    console.error("Push sender failed", error instanceof Error ? error.message : "unknown error");
    return new Response("Push sender failed", { status: 500 });
  }
});

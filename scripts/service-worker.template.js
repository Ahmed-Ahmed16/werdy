const CACHE_NAME = "werdy-__VERSION__";
const APP_ROOT = "/werdy/";
const PRECACHE = __PRECACHE__;
// Fonts come from Google, so they cannot be in PRECACHE. They are cached on
// first use, under a name that survives app updates.
const FONT_CACHE = "fonts-v1";
const FONT_ORIGINS = ["https://fonts.googleapis.com", "https://fonts.gstatic.com"];
// A weak signal can keep a request open for a minute. After this long the
// cached app opens instead.
const NETWORK_TIMEOUT_MS = 3000;

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(PRECACHE))
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((names) => Promise.all(names
        .filter((name) => name.startsWith("werdy-") && name !== CACHE_NAME)
        .map((name) => caches.delete(name))))
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET") {
    return;
  }
  if (FONT_ORIGINS.includes(url.origin)) {
    event.respondWith(fontResponse(request));
    return;
  }
  if (url.origin !== self.location.origin || !url.pathname.startsWith(APP_ROOT)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(
      withTimeout(fetch(request), NETWORK_TIMEOUT_MS)
        .catch(() => caches.match(APP_ROOT + "index.html"))
        .then((response) => response || fetch(request))
    );
    return;
  }

  event.respondWith(
    caches.match(request).then((cached) => cached || fetch(request))
  );
});

// Serves the cached font at once and refreshes it in the background.
async function fontResponse(request) {
  const cache = await caches.open(FONT_CACHE);
  const cached = await cache.match(request);
  const fresh = fetch(request).then((response) => {
    if (response.ok || response.type === "opaque") {
      return cache.put(request, response.clone()).then(() => response);
    }
    return response;
  });
  if (cached) {
    fresh.catch(() => {});
    return cached;
  }
  return fresh;
}

function withTimeout(promise, milliseconds) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("network timeout")), milliseconds);
    promise.then(
      (value) => { clearTimeout(timer); resolve(value); },
      (error) => { clearTimeout(timer); reject(error); }
    );
  });
}

self.addEventListener("push", (event) => {
  let payload;
  try {
    payload = event.data?.json();
  } catch (_error) {
    return;
  }
  if (!payload?.notification) return;
  const { title, body, navigate, tag } = payload.notification;
  event.waitUntil(self.registration.showNotification(title, {
    body,
    tag,
    data: { url: navigate },
  }));
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  event.waitUntil((async () => {
    const windows = await clients.matchAll({ type: "window", includeUncontrolled: true });
    const open = windows.find((client) => {
      const url = new URL(client.url);
      return url.origin === self.location.origin && url.pathname.startsWith(APP_ROOT);
    });
    if (open) return open.focus();
    return clients.openWindow(APP_ROOT);
  })());
});

self.addEventListener("pushsubscriptionchange", (_event) => {
  // The next app open repairs the subscription with the owner's session.
});

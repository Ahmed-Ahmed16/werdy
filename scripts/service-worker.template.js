const CACHE_NAME = "werdy-__VERSION__";
const APP_ROOT = "/werdy/";
const PRECACHE = __PRECACHE__;

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
  if (request.method !== "GET" || url.origin !== self.location.origin ||
      !url.pathname.startsWith(APP_ROOT)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request).catch(() => caches.match(APP_ROOT + "index.html"))
    );
    return;
  }

  event.respondWith(
    caches.match(request).then((cached) => cached || fetch(request))
  );
});

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

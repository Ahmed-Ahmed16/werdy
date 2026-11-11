const supported = "serviceWorker" in navigator && "PushManager" in window && "Notification" in window;
const ios = /iPhone|iPad|iPod/.test(navigator.userAgent)
    || (/Macintosh/.test(navigator.userAgent) && navigator.maxTouchPoints > 1);
const standalone = matchMedia("(display-mode: standalone)").matches || navigator.standalone === true;
if (operation === "status") {
    return { supported, ios, standalone, permission: supported ? Notification.permission : "default" };
}
if (!supported) return operation === "unsubscribe" ? true : null;
if (operation === "subscribe" && await Notification.requestPermission() !== "granted") return null;

let timer;
let registration;
let timedOut = false;
const timeoutError = new Error("push operation timed out after 10 seconds");
try {
    return await Promise.race([
        (async () => {
            if (operation === "subscribe") {
                const base64 = key.replace(/-/g, "+").replace(/_/g, "/");
                const raw = atob(base64.padEnd(Math.ceil(base64.length / 4) * 4, "="));
                const applicationServerKey = Uint8Array.from(raw, character => character.charCodeAt(0));
                registration = await navigator.serviceWorker.ready;
                if (timedOut) return null;
                const subscription = await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey });
                if (timedOut) {
                    await subscription.unsubscribe();
                    return null;
                }
                const json = subscription.toJSON();
                return { endpoint: json.endpoint, p256dh: json.keys.p256dh, auth: json.keys.auth };
            }
            registration = await navigator.serviceWorker.getRegistration("/werdy/");
            if (!registration) return operation === "unsubscribe" ? true : null;
            const subscription = await registration.pushManager.getSubscription();
            if (operation === "unsubscribe") {
                if (subscription) await subscription.unsubscribe();
                return true;
            }
            if (!subscription) return null;
            const json = subscription.toJSON();
            return { endpoint: json.endpoint, p256dh: json.keys.p256dh, auth: json.keys.auth };
        })(),
        new Promise((_, reject) => {
            timer = setTimeout(() => {
                timedOut = true;
                reject(timeoutError);
            }, 10_000);
        })
    ]);
} catch (error) {
    if (operation === "subscribe" && error === timeoutError && registration) {
        const subscription = await registration.pushManager.getSubscription();
        if (subscription) await subscription.unsubscribe();
    }
    throw error;
} finally {
    clearTimeout(timer);
}

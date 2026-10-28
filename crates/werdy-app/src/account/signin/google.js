let active = true;
let timer;
let finish;
const outcome = new Promise(resolve => { finish = resolve; });
const stop = dioxus.recv().then(() => { active = false; finish(); });
const fail = () => {
    if (active) dioxus.send({state: "error"});
    active = false;
    finish();
};
timer = setTimeout(fail, 120000);
try {
    const ready = globalThis.google?.accounts?.id ? Promise.resolve() :
        (globalThis.werdyGisReady ??= new Promise((resolve, reject) => {
            const script = document.createElement("script");
            script.src = "https://accounts.google.com/gsi/client";
            script.async = true;
            script.onload = resolve;
            script.onerror = () => { delete globalThis.werdyGisReady; reject(); };
            document.head.appendChild(script);
        }));
    await Promise.race([ready, outcome]);
    if (active) {
        const container = document.getElementById("werdy-google-signin");
        if (!container) throw new Error();
        google.accounts.id.initialize({client_id: clientId, nonce, auto_select: false,
            callback: reply => {
                if (!active) return;
                if (typeof reply?.credential !== "string" || !reply.credential) return fail();
                dioxus.send({state: "ok", idToken: reply.credential});
                active = false;
                finish();
            }});
        google.accounts.id.renderButton(container, {theme: "outline", size: "large", locale: container.closest("[lang]")?.lang || "ar"});
    }
    await Promise.race([outcome, stop]);
} catch { fail(); }
finally { active = false; clearTimeout(timer); }

/* Service worker ScolIA : notifications Web Push uniquement (aucun cache, pas d'interception fetch). */
const ICON = "/Logo%20header.png";

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

function safeUrl(raw) {
  try {
    if (typeof raw !== "string" || !raw.startsWith("/") || raw.startsWith("//")) return "/";
    return raw;
  } catch {
    return "/";
  }
}

self.addEventListener("push", (event) => {
  let data = {};
  try {
    data = event.data ? event.data.json() : {};
  } catch {
    data = {};
  }
  const payload = {
    kind: typeof data.kind === "string" ? data.kind : "signal",
    title: typeof data.title === "string" && data.title ? data.title : "ScolIA",
    body: typeof data.body === "string" ? data.body : "",
    url: safeUrl(data.url),
    tag: typeof data.tag === "string" && data.tag ? data.tag : undefined,
  };
  event.waitUntil(
    (async () => {
      const wins = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
      const active = wins.find((w) => w.visibilityState === "visible" && w.focused);
      if (active) {
        active.postMessage({ type: "scolia-push", payload });
        return;
      }
      await self.registration.showNotification(payload.title, {
        body: payload.body,
        tag: payload.tag,
        renotify: Boolean(payload.tag),
        icon: ICON,
        badge: ICON,
        data: { url: payload.url },
      });
    })(),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const url = safeUrl(event.notification.data && event.notification.data.url);
  event.waitUntil(
    (async () => {
      const wins = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
      const target = new URL(url, self.location.origin).href;
      for (const w of wins) {
        if (new URL(w.url).origin !== self.location.origin) continue;
        try {
          await w.focus();
          if ("navigate" in w) await w.navigate(target);
          return;
        } catch {
          /* essayer le suivant */
        }
      }
      await self.clients.openWindow(target);
    })(),
  );
});

self.addEventListener("pushsubscriptionchange", (event) => {
  event.waitUntil(
    (async () => {
      try {
        const res = await fetch("/api/push/config", { credentials: "same-origin" });
        const cfg = await res.json();
        if (!cfg.configured || !cfg.publicKey) return;
        const old = event.oldSubscription;
        const key = old && old.options && old.options.applicationServerKey;
        const sub =
          event.newSubscription ||
          (await self.registration.pushManager.subscribe({
            userVisibleOnly: true,
            applicationServerKey: key || cfg.publicKey,
          }));
        await fetch("/api/push/subscribe", {
          method: "POST",
          credentials: "same-origin",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(sub.toJSON()),
        });
      } catch {
        /* best effort */
      }
    })(),
  );
});

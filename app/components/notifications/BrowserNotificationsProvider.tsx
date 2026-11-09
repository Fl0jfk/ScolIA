"use client";

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { useAppUser } from "@/app/hooks/useAppUser";
import { useDashboardSignalsContext } from "@/app/components/shell/DashboardSignalsProvider";
import {
  findIncreasedSignals,
  isInQuietHours,
  minuteToTime,
  parisMinuteNow,
  safeInternalUrl,
  timeToMinute,
  toSignalCounts,
  urlBase64ToUint8Array,
  type SignalCounts,
} from "@/app/components/notifications/notification-utils";
import type { WebPushKind } from "@/app/lib/web-push/types";

type Permission = "default" | "granted" | "denied" | "unsupported" | "ios-install";

type Prefs = {
  mutedKinds: WebPushKind[];
  quietStartMinute: number | null;
  quietEndMinute: number | null;
};

const DEFAULT_PREFS: Prefs = { mutedKinds: [], quietStartMinute: null, quietEndMinute: null };
const DISMISS_MS = 7 * 24 * 60 * 60 * 1000;
const ICON = "/Logo%20header.png";
const CARD =
  "pointer-events-auto rounded-2xl bg-white p-4 shadow-xl ring-1 ring-black/10 text-[var(--dash-ink)]";

function lsGet(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}
function lsSet(key: string, value: string | null) {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, value);
  } catch {
    /* stockage indisponible */
  }
}

function isStaff(
  user: { roles: string[]; orgAdmin: boolean; platformAdmin: boolean } | null | undefined,
): boolean {
  if (!user) return false;
  if (user.orgAdmin || user.platformAdmin) return true;
  const visible = user.roles.filter((r) => r !== "master");
  if (visible.length === 0) return user.roles.includes("master");
  return visible.some((r) => r !== "parent" && r !== "eleve");
}

function detectPermission(): Permission {
  if (typeof window === "undefined") return "unsupported";
  if (typeof Notification === "undefined") {
    const ios =
      /iPad|iPhone|iPod/.test(navigator.userAgent) ||
      (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1);
    return ios ? "ios-install" : "unsupported";
  }
  if (!("serviceWorker" in navigator)) return "unsupported";
  return Notification.permission as Permission;
}

function tabIsActive(): boolean {
  return document.visibilityState === "visible" && document.hasFocus();
}

export function BrowserNotificationsProvider({ children }: { children: ReactNode }) {
  const { isLoaded, user } = useAppUser();
  const staff = isLoaded && isStaff(user);
  return (
    <>
      {children}
      {staff && user ? <NotificationsRuntime userId={user.id} /> : null}
    </>
  );
}

function NotificationsRuntime({ userId }: { userId: string }) {
  const { notifications, fetchGeneration, refresh } = useDashboardSignalsContext();
  const [permission, setPermission] = useState<Permission>("unsupported");
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [publicKey, setPublicKey] = useState<string | null>(null);
  const [signalsServer, setSignalsServer] = useState(false);
  const [registration, setRegistration] = useState<ServiceWorkerRegistration | null>(null);
  const [subscribed, setSubscribed] = useState(false);
  const [deviceOff, setDeviceOff] = useState(false);
  const [prefs, setPrefs] = useState<Prefs>(DEFAULT_PREFS);
  const [promptHidden, setPromptHidden] = useState(true);
  const [panelOpen, setPanelOpen] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const offKey = `scolia:notif:device-off:${userId}`;
  const dismissKey = `scolia:notif:dismissed:${userId}`;
  const baselineKey = `scolia:notif:signals:${userId}`;

  const prefsRef = useRef(prefs);
  prefsRef.current = prefs;
  const deviceOffRef = useRef(deviceOff);
  deviceOffRef.current = deviceOff;

  const unavailable = permission === "unsupported" || permission === "ios-install";

  // Initialisation : permission, réglage local, rappel « plus tard ».
  useEffect(() => {
    setPermission(detectPermission());
    setDeviceOff(lsGet(offKey) === "1");
    const dismissedAt = Number(lsGet(dismissKey) ?? "0");
    setPromptHidden(Boolean(dismissedAt) && Date.now() - dismissedAt < DISMISS_MS);
  }, [offKey, dismissKey]);

  // Service worker + config serveur + préférences.
  useEffect(() => {
    if (unavailable) return;
    let cancelled = false;
    void (async () => {
      try {
        const reg = await navigator.serviceWorker.register("/sw.js", { scope: "/" });
        if (!cancelled) setRegistration(reg);
      } catch {
        /* SW indisponible (navigation privée, etc.) */
      }
      try {
        const res = await fetch("/api/push/config", { cache: "no-store" });
        if (res.ok) {
          const cfg = (await res.json()) as {
            configured: boolean;
            publicKey: string | null;
            signalsServer?: boolean;
          };
          if (!cancelled) {
            setConfigured(Boolean(cfg.configured && cfg.publicKey));
            setPublicKey(cfg.publicKey);
            setSignalsServer(Boolean(cfg.signalsServer));
          }
        } else if (!cancelled) setConfigured(false);
      } catch {
        if (!cancelled) setConfigured(false);
      }
      try {
        const res = await fetch("/api/push/preferences", { cache: "no-store" });
        if (res.ok && !cancelled) setPrefs((await res.json()) as Prefs);
      } catch {
        /* valeurs par défaut */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [unavailable]);

  // Abonnement Web Push quand tout est réuni.
  useEffect(() => {
    if (permission !== "granted" || !registration) return;
    let cancelled = false;
    void (async () => {
      try {
        const existing = await registration.pushManager.getSubscription();
        if (deviceOff) {
          if (existing) {
            await fetch("/api/push/unsubscribe", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ endpoint: existing.endpoint }),
            }).catch(() => undefined);
            await existing.unsubscribe().catch(() => undefined);
          }
          if (!cancelled) setSubscribed(false);
          return;
        }
        if (!configured || !publicKey) return;
        const sub =
          existing ??
          (await registration.pushManager.subscribe({
            userVisibleOnly: true,
            applicationServerKey: urlBase64ToUint8Array(publicKey),
          }));
        const res = await fetch("/api/push/subscribe", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(sub.toJSON()),
        });
        if (!cancelled) setSubscribed(res.ok);
      } catch {
        if (!cancelled) setSubscribed(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [permission, registration, configured, publicKey, deviceOff]);

  const canNotifyLocally = useCallback((kind: WebPushKind): boolean => {
    if (deviceOffRef.current) return false;
    if (typeof Notification === "undefined" || Notification.permission !== "granted") return false;
    const p = prefsRef.current;
    if (p.mutedKinds.includes(kind)) return false;
    if (isInQuietHours(parisMinuteNow(), p.quietStartMinute, p.quietEndMinute)) return false;
    return !tabIsActive();
  }, []);

  const showLocal = useCallback(
    async (title: string, body: string, url: string, tag: string) => {
      try {
        const reg = registration ?? (await navigator.serviceWorker.ready);
        await reg.showNotification(title, {
          body,
          tag,
          renotify: true,
          icon: ICON,
          badge: ICON,
          data: { url: safeInternalUrl(url) },
        } as NotificationOptions);
      } catch {
        /* best effort */
      }
    },
    [registration],
  );

  // Nouveautés de l'ENT : compteur en hausse -> notification locale.
  useEffect(() => {
    if (fetchGeneration === 0) return; // pas encore de réponse réseau : ne pas fausser la référence
    let previous: SignalCounts | null = null;
    const raw = lsGet(baselineKey);
    if (raw) {
      try {
        previous = JSON.parse(raw) as SignalCounts;
      } catch {
        previous = null;
      }
    }
    const increased = findIncreasedSignals(previous, notifications);
    lsSet(baselineKey, JSON.stringify(toSignalCounts(notifications)));
    // Le serveur notifie (même sans onglet) : pas de doublon local si cet appareil est abonné.
    if (signalsServer && subscribed) return;
    if (!registration || increased.length === 0 || !canNotifyLocally("signal")) return;

    for (const n of increased) {
      // Un seul onglet notifie : revendication horodatée par (signal, compteur).
      const claimKey = `scolia:notif:claim:${userId}:${n.id}:${n.count}`;
      const token = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
      const existing = Number(lsGet(claimKey)?.split("-")[0] ?? "0");
      if (existing && Date.now() - existing < 120_000) continue;
      lsSet(claimKey, token);
      if (lsGet(claimKey) !== token) continue;
      window.setTimeout(() => lsSet(claimKey, null), 150_000);
      void showLocal(n.label, n.detail || "Nouveauté dans ScolIA", n.href, `signal-${n.id}`);
    }
  }, [
    fetchGeneration,
    notifications,
    registration,
    baselineKey,
    userId,
    canNotifyLocally,
    showLocal,
    signalsServer,
    subscribed,
  ]);

  // Onglet masqué : le provider de signaux ne rafraîchit plus, on le fait ici (~60 s).
  useEffect(() => {
    if (permission !== "granted" || deviceOff) return;
    const id = window.setInterval(() => {
      if (document.hidden) void refresh();
    }, 60_000);
    return () => window.clearInterval(id);
  }, [permission, deviceOff, refresh]);

  // Repli messages sans Web Push serveur.
  useEffect(() => {
    if (permission !== "granted" || configured !== false) return;
    const handler = (e: Event) => {
      const d = (e as CustomEvent).detail as
        | { type?: string; conversationId?: string; message?: { senderId?: string } }
        | undefined;
      if (!d || d.type !== "message" || !d.message?.senderId || d.message.senderId === userId) return;
      if (!canNotifyLocally("messagerie")) return;
      const url = d.conversationId
        ? `/messagerie?c=${encodeURIComponent(d.conversationId)}`
        : "/messagerie";
      void showLocal(
        "Nouveau message",
        "Vous avez reçu un message dans ScolIA.",
        url,
        `msg-${d.conversationId ?? "x"}`,
      );
    };
    window.addEventListener("scolia-messaging-event", handler);
    return () => window.removeEventListener("scolia-messaging-event", handler);
  }, [permission, configured, userId, canNotifyLocally, showLocal]);

  const enable = useCallback(async () => {
    setMessage(null);
    lsSet(offKey, null);
    setDeviceOff(false);
    if (typeof Notification === "undefined") return;
    try {
      const result = await Notification.requestPermission();
      setPermission(result as Permission);
      if (result === "denied") setMessage("Notifications refusées par le navigateur.");
    } catch {
      setMessage("Impossible de demander l'autorisation.");
    }
  }, [offKey]);

  const dismissPrompt = useCallback(() => {
    lsSet(dismissKey, String(Date.now()));
    setPromptHidden(true);
  }, [dismissKey]);

  const disableDevice = useCallback(() => {
    lsSet(offKey, "1");
    setDeviceOff(true);
  }, [offKey]);

  const savePrefs = useCallback(async (next: Prefs) => {
    setPrefs(next);
    try {
      await fetch("/api/push/preferences", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(next),
      });
    } catch {
      setMessage("Préférences non enregistrées.");
    }
  }, []);

  const sendTest = useCallback(async () => {
    setMessage(null);
    if (configured && subscribed) {
      const res = await fetch("/api/push/test", { method: "POST" }).catch(() => null);
      setMessage(
        res?.ok
          ? "Notification de test envoyée (masquée si l'onglet est actif)."
          : "Envoi impossible.",
      );
      return;
    }
    await showLocal(
      "Notifications activées ✅",
      "Vous serez prévenu(e) des nouveautés de ScolIA.",
      "/",
      "push-test",
    );
    setMessage("Notification de test envoyée.");
  }, [configured, subscribed, showLocal]);

  const active = permission === "granted" && !deviceOff;
  const showPrompt =
    !promptHidden && configured !== null && (permission === "default" || permission === "ios-install");
  const quietOn = prefs.quietStartMinute !== null && prefs.quietEndMinute !== null;

  const toggleKind = (kind: WebPushKind, on: boolean) => {
    const set = new Set(prefs.mutedKinds);
    if (on) set.delete(kind);
    else set.add(kind);
    void savePrefs({ ...prefs, mutedKinds: [...set] });
  };

  if (permission === "unsupported") return null;

  return (
    <div className="pointer-events-none fixed bottom-4 left-4 z-[60] flex max-w-[calc(100vw-2rem)] flex-col items-start gap-2 lg:left-[18.5rem] print:hidden">
      {showPrompt ? (
        <div className={`${CARD} w-80`} role="dialog" aria-label="Notifications">
          {permission === "ios-install" ? (
            <p className="text-sm">
              Ajoutez ScolIA à l&apos;écran d&apos;accueil pour recevoir les notifications.
            </p>
          ) : (
            <p className="text-sm">
              Être prévenu des nouveaux messages et des nouveautés, même quand l&apos;onglet est en
              arrière-plan ?
            </p>
          )}
          <div className="mt-3 flex gap-2">
            {permission === "default" ? (
              <button
                type="button"
                onClick={() => void enable()}
                className="rounded-full bg-[var(--dash-lime)] px-4 py-1.5 text-sm font-bold text-[var(--dash-ink)]"
              >
                Activer
              </button>
            ) : null}
            <button
              type="button"
              onClick={dismissPrompt}
              className="rounded-full px-4 py-1.5 text-sm font-semibold text-[var(--dash-ink)]/70 hover:bg-black/5"
            >
              Plus tard
            </button>
          </div>
        </div>
      ) : null}

      {panelOpen ? (
        <div className={`${CARD} w-80 space-y-3`} role="dialog" aria-label="Réglages des notifications">
          <p className="text-sm font-bold">Notifications du navigateur</p>
          {permission === "denied" ? (
            <p className="text-sm">
              Notifications bloquées : réactivez-les dans les réglages du navigateur (icône cadenas près
              de l&apos;adresse).
            </p>
          ) : permission === "ios-install" ? (
            <p className="text-sm">
              Ajoutez ScolIA à l&apos;écran d&apos;accueil pour recevoir les notifications.
            </p>
          ) : (
            <>
              <label className="flex items-center justify-between gap-3 text-sm">
                <span>Activées sur cet appareil</span>
                <input
                  type="checkbox"
                  checked={active}
                  onChange={(e) => {
                    if (e.target.checked) void enable();
                    else disableDevice();
                  }}
                />
              </label>
              {active ? (
                <>
                  <fieldset className="space-y-1.5 text-sm">
                    <legend className="mb-1 font-semibold">Catégories</legend>
                    {(
                      [
                        ["messagerie", "Messages"],
                        ["messagerie_mention", "Mentions"],
                        ["signal", "Nouveautés de l'ENT"],
                      ] as [WebPushKind, string][]
                    ).map(([kind, label]) => (
                      <label key={kind} className="flex items-center justify-between gap-3">
                        <span>{label}</span>
                        <input
                          type="checkbox"
                          checked={!prefs.mutedKinds.includes(kind)}
                          onChange={(e) => toggleKind(kind, e.target.checked)}
                        />
                      </label>
                    ))}
                  </fieldset>
                  <div className="space-y-1.5 text-sm">
                    <label className="flex items-center justify-between gap-3">
                      <span className="font-semibold">Heures calmes</span>
                      <input
                        type="checkbox"
                        checked={quietOn}
                        onChange={(e) =>
                          void savePrefs({
                            ...prefs,
                            quietStartMinute: e.target.checked ? 19 * 60 : null,
                            quietEndMinute: e.target.checked ? 7 * 60 + 30 : null,
                          })
                        }
                      />
                    </label>
                    {quietOn ? (
                      <div className="flex items-center gap-2">
                        <input
                          type="time"
                          className="rounded-lg border border-black/10 px-2 py-1"
                          value={minuteToTime(prefs.quietStartMinute ?? 0)}
                          onChange={(e) => {
                            const m = timeToMinute(e.target.value);
                            if (m !== null) void savePrefs({ ...prefs, quietStartMinute: m });
                          }}
                        />
                        <span>à</span>
                        <input
                          type="time"
                          className="rounded-lg border border-black/10 px-2 py-1"
                          value={minuteToTime(prefs.quietEndMinute ?? 0)}
                          onChange={(e) => {
                            const m = timeToMinute(e.target.value);
                            if (m !== null) void savePrefs({ ...prefs, quietEndMinute: m });
                          }}
                        />
                      </div>
                    ) : null}
                  </div>
                  <button
                    type="button"
                    onClick={() => void sendTest()}
                    className="rounded-full bg-[var(--dash-lime)] px-4 py-1.5 text-sm font-bold text-[var(--dash-ink)]"
                  >
                    Envoyer une notification de test
                  </button>
                  {configured === false ? (
                    <p className="text-xs text-[var(--dash-ink)]/60">
                      Mode local : les notifications arrivent tant qu&apos;un onglet ScolIA reste ouvert.
                    </p>
                  ) : null}
                </>
              ) : null}
            </>
          )}
          {message ? <p className="text-xs text-[var(--dash-ink)]/70">{message}</p> : null}
        </div>
      ) : null}

      <button
        type="button"
        onClick={() => setPanelOpen((v) => !v)}
        aria-label="Réglages des notifications"
        aria-expanded={panelOpen}
        title="Notifications"
        className="pointer-events-auto flex h-9 w-9 items-center justify-center rounded-full bg-white text-[var(--dash-ink)] shadow-md ring-1 ring-black/10 hover:bg-black/5"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
          <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
          {active ? null : <path d="M3 3l18 18" />}
        </svg>
      </button>
    </div>
  );
}

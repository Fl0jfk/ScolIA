"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  RdvInscriptionBookingRow,
  RdvInscriptionConfigPublic,
  RdvInscriptionDirectionRow,
  RdvInscriptionSlot,
} from "@/app/lib/rdv-inscription-types";

type AdminPayload = {
  config: RdvInscriptionConfigPublic;
  directions: RdvInscriptionDirectionRow[];
  google: {
    linked: boolean;
    linkedEmail: string | null;
    linkedDisplayName: string | null;
    linkedAt: string | null;
    clientConfigured: boolean;
  };
  publicLinks: Array<{ slug: string; label: string; url: string }>;
  oauthRedirectUri: string | null;
  oauthStartPath: string;
};

function formatSlot(isoStart: string, isoEnd: string): string {
  const s = new Date(isoStart);
  const e = new Date(isoEnd);
  const day = s.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "short",
    day: "numeric",
    month: "short",
  });
  const hm = (d: Date) =>
    d.toLocaleTimeString("fr-FR", { timeZone: "Europe/Paris", hour: "2-digit", minute: "2-digit" });
  return `${day} ${hm(s)}–${hm(e)}`;
}

export default function RdvInscriptionAdminClient() {
  const [data, setData] = useState<AdminPayload | null>(null);
  const [bookings, setBookings] = useState<RdvInscriptionBookingRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [testSlots, setTestSlots] = useState<RdvInscriptionSlot[] | null>(null);
  const [sampleTitles, setSampleTitles] = useState<string[]>([]);
  const [copied, setCopied] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [resAll, resBook] = await Promise.all([
        fetch("/api/rdv-inscription/admin"),
        fetch("/api/rdv-inscription/admin?view=bookings"),
      ]);
      const all = (await resAll.json()) as AdminPayload & { error?: string };
      const book = (await resBook.json()) as { bookings?: RdvInscriptionBookingRow[]; error?: string };
      if (!resAll.ok) throw new Error(all.error || "Chargement impossible.");
      setData(all);
      setBookings(book.bookings || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load().then(() => {
      const sp = new URLSearchParams(window.location.search);
      const g = sp.get("google");
      if (g === "linked") {
        setMessage("Compte Google connecté.");
        const url = new URL(window.location.href);
        url.searchParams.delete("google");
        url.searchParams.delete("detail");
        window.history.replaceState({}, "", url.pathname + url.search);
      }
      if (g === "forbidden") setError("Droit insuffisant pour lier Google.");
      if (g === "error") setError(sp.get("detail") || "Erreur OAuth Google.");
    });
  }, [load]);

  async function put(body: Record<string, unknown>) {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/rdv-inscription/admin", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = (await res.json()) as { error?: string; success?: boolean };
      if (!res.ok) throw new Error(json.error || "Échec.");
      setMessage("Enregistré.");
      await load();
      return json;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }

  async function copyLink(url: string) {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(url);
      setTimeout(() => setCopied(null), 2000);
    } catch {
      setError("Impossible de copier le lien.");
    }
  }

  if (loading && !data) {
    return <p className="p-6 text-slate-600">Chargement…</p>;
  }
  if (!data) {
    return (
      <div className="p-6">
        <p className="text-red-700">{error || "Erreur"}</p>
        <button type="button" className="mt-3 underline" onClick={() => void load()}>
          Réessayer
        </button>
      </div>
    );
  }

  const { config, directions, google, publicLinks, oauthRedirectUri, oauthStartPath } = data;

  return (
    <div className="mx-auto max-w-4xl space-y-8 p-4 sm:p-6">
      <header>
        <h1 className="text-2xl font-bold text-slate-900">RDV inscription direction</h1>
        <p className="mt-1 text-sm text-slate-600">
          Chaque direction a son agenda Google, son texte de page, son motif de titre et son
          e-mail secrétariat.
        </p>
      </header>

      {message ? (
        <p className="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
          {message}
        </p>
      ) : null}
      {error ? (
        <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </p>
      ) : null}

      <section className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
        <h2 className="text-lg font-bold text-slate-900">Compte Google technique</h2>
        <p className="mt-1 text-sm text-slate-600">
          Connectez un compte Google, puis demandez à chaque directrice de{" "}
          <strong>partager son agenda</strong> avec ce compte (droits de modification des
          événements).
        </p>
        {!google.clientConfigured ? (
          <p className="mt-3 rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-900">
            Configurez <code className="font-mono text-xs">GOOGLE_CLIENT_ID</code> et{" "}
            <code className="font-mono text-xs">GOOGLE_CLIENT_SECRET</code> (ou secrets.google du
            tenant). URI de redirection à enregistrer dans Google Cloud :{" "}
            <code className="break-all font-mono text-xs">
              {oauthRedirectUri || "…/api/rdv-inscription/oauth/callback"}
            </code>
          </p>
        ) : null}
        <div className="mt-4 flex flex-wrap items-center gap-3">
          {google.linked ? (
            <>
              <span className="inline-flex items-center gap-1 rounded-full bg-emerald-100 px-3 py-1 text-sm font-semibold text-emerald-900">
                Lié
                {google.linkedEmail ? ` — ${google.linkedEmail}` : ""}
              </span>
              <button
                type="button"
                disabled={busy}
                onClick={() => void put({ action: "unlink-google" })}
                className="inline-flex items-center gap-1 rounded-md border border-slate-300 px-3 py-1.5 text-sm font-semibold text-slate-700 hover:bg-slate-50"
              >
                Déconnecter
              </button>
              <a
                href={oauthStartPath}
                className="inline-flex items-center gap-1 rounded-md border border-slate-300 px-3 py-1.5 text-sm font-semibold text-slate-700 hover:bg-slate-50"
              >
                Reconnecter
              </a>
            </>
          ) : (
            <>
              <a
                href={oauthStartPath}
                className="inline-flex items-center gap-1 rounded-md bg-sky-700 px-4 py-2 text-sm font-bold text-white hover:bg-sky-800"
              >
                Connecter Google Agenda
              </a>
              {config.googleLinked ? (
                <p className="w-full text-sm text-amber-800">
                  Une ancienne connexion a été détectée mais le jeton Google n’est plus
                  utilisable. Cliquez sur <strong>Connecter Google Agenda</strong> pour
                  renouveler l’accès, puis retestez les créneaux.
                </p>
              ) : null}
            </>
          )}
        </div>
        <form
          className="mt-4"
          onSubmit={(e) => {
            e.preventDefault();
            const fd = new FormData(e.currentTarget);
            void put({
              action: "save-config",
              config: { enabled: fd.get("enabled") === "on" },
            });
          }}
        >
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" name="enabled" defaultChecked={config.enabled} />
            <span className="font-semibold">Coupe-circuit global (désactiver toutes les pages)</span>
          </label>
          <button
            type="submit"
            disabled={busy}
            className="mt-2 rounded-md border border-slate-300 px-3 py-1.5 text-sm font-semibold disabled:opacity-60"
          >
            Enregistrer
          </button>
        </form>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
        <h2 className="text-lg font-bold text-slate-900">Directions & agendas</h2>
        <p className="mt-1 text-sm text-slate-600">
          Paramétrez chaque direction indépendamment (agenda, textes, motif Google, notif
          secrétariat, horizon).
        </p>
        <div className="mt-4 space-y-6">
          {directions.map((d) => (
            <form
              key={`${d.id}-${d.eventTitlePattern}-${d.notifyEmail || ""}-${d.horizonDays}-${d.title}`}
              className="grid gap-2 rounded-lg border border-slate-100 bg-slate-50 p-3 sm:grid-cols-2"
              onSubmit={(e) => {
                e.preventDefault();
                const fd = new FormData(e.currentTarget);
                void put({
                  action: "save-direction",
                  direction: {
                    id: d.id,
                    slug: String(fd.get("slug") || d.slug),
                    label: String(fd.get("label") || ""),
                    googleCalendarId: String(fd.get("googleCalendarId") || ""),
                    directriceDisplayName: String(fd.get("directriceDisplayName") || "") || null,
                    title: String(fd.get("title") || ""),
                    intro: String(fd.get("intro") || ""),
                    eventTitlePattern: String(fd.get("eventTitlePattern") || ""),
                    notifyEmail: String(fd.get("notifyEmail") || "") || null,
                    location: String(fd.get("location") || ""),
                    consentLabel: String(fd.get("consentLabel") || ""),
                    horizonDays: Number(fd.get("horizonDays") || 60),
                    active: fd.get("active") === "on",
                    sortOrder: Number(fd.get("sortOrder") || d.sortOrder),
                  },
                });
              }}
            >
              <div className="sm:col-span-2 border-b border-slate-200 pb-2">
                <p className="text-base font-bold text-slate-900">{d.label}</p>
              </div>
              <label className="block text-sm">
                <span className="font-semibold">Libellé</span>
                <input
                  name="label"
                  defaultValue={d.label}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm">
                <span className="font-semibold">Slug URL</span>
                <input
                  name="slug"
                  defaultValue={d.slug}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 font-mono text-xs"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold">Google Calendar ID</span>
                <input
                  name="googleCalendarId"
                  defaultValue={d.googleCalendarId}
                  placeholder="directrice@ecole.fr"
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 font-mono text-xs"
                />
              </label>
              <label className="block text-sm">
                <span className="font-semibold">Nom directrice (affiché)</span>
                <input
                  name="directriceDisplayName"
                  defaultValue={d.directriceDisplayName || ""}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm">
                <span className="font-semibold">Ordre</span>
                <input
                  name="sortOrder"
                  type="number"
                  defaultValue={d.sortOrder}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold">Titre page publique</span>
                <input
                  name="title"
                  defaultValue={d.title}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold">Introduction</span>
                <textarea
                  name="intro"
                  rows={2}
                  defaultValue={d.intro}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold">Texte recherché dans le titre Google</span>
                <input
                  name="eventTitlePattern"
                  defaultValue={d.eventTitlePattern}
                  placeholder="RDV inscription"
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
                <span className="mt-1 block text-xs text-slate-500">
                  Exactement ce que cette directrice écrit dans le titre (casse/accents ignorés).
                  Plusieurs formulations :{" "}
                  <code className="font-mono">RDV inscription | rendez-vous inscription</code>.
                </span>
              </label>
              <label className="block text-sm">
                <span className="font-semibold">Horizon (jours)</span>
                <input
                  name="horizonDays"
                  type="number"
                  min={7}
                  max={180}
                  defaultValue={d.horizonDays}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm">
                <span className="font-semibold">E-mail notif secrétariat</span>
                <input
                  name="notifyEmail"
                  type="email"
                  defaultValue={d.notifyEmail || ""}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm">
                <span className="font-semibold">Lieu</span>
                <input
                  name="location"
                  defaultValue={d.location}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="block text-sm sm:col-span-2">
                <span className="font-semibold">Libellé consentement</span>
                <input
                  name="consentLabel"
                  defaultValue={d.consentLabel}
                  className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2"
                />
              </label>
              <label className="flex items-center gap-2 text-sm sm:col-span-2">
                <input type="checkbox" name="active" defaultChecked={d.active} />
                <span className="font-semibold">Direction active (page publique)</span>
              </label>
              <div className="flex flex-wrap gap-2 sm:col-span-2">
                <button
                  type="submit"
                  disabled={busy}
                  className="rounded-md bg-slate-900 px-3 py-1.5 text-sm font-bold text-white disabled:opacity-60"
                >
                  Enregistrer
                </button>
                <button
                  type="button"
                  disabled={busy || !google.linked || !d.googleCalendarId}
                  className="inline-flex items-center gap-1 rounded-md border border-slate-300 px-3 py-1.5 text-sm font-semibold disabled:opacity-50"
                  onClick={async () => {
                    setTestSlots(null);
                    setSampleTitles([]);
                    const json = await put({ action: "test-slots", directionId: d.id });
                    if (json && "slots" in json) {
                      const payload = json as {
                        slots: RdvInscriptionSlot[];
                        count?: number;
                        titlePattern?: string;
                        upcomingEventCount?: number;
                        sampleTitles?: string[];
                      };
                      setTestSlots(payload.slots);
                      setSampleTitles(payload.sampleTitles || []);
                      const count = payload.count ?? 0;
                      const upcoming = payload.upcomingEventCount ?? 0;
                      const motif = payload.titlePattern || d.eventTitlePattern;
                      if (count > 0) {
                        setMessage(
                          `${count} créneau(x) libre(s) sur « ${d.label} » (motif « ${motif} »).`,
                        );
                      } else if (upcoming === 0) {
                        setMessage(
                          `0 créneau sur « ${d.label} » : aucun événement à venir sur cet agenda dans l’horizon. Vérifiez le Calendar ID et le partage avec le compte Google lié.`,
                        );
                      } else {
                        setMessage(
                          `0 créneau libre sur « ${d.label} » pour le motif « ${motif} », alors que ${upcoming} événement(s) à venir ont été lus. Adaptez le texte recherché de cette direction.`,
                        );
                      }
                    }
                  }}
                >
                  Tester les créneaux
                </button>
              </div>
            </form>
          ))}
        </div>
        {testSlots && testSlots.length > 0 ? (
          <ul className="mt-4 space-y-1 text-sm text-slate-700">
            {testSlots.map((s) => (
              <li key={s.eventId}>
                {formatSlot(s.startAt, s.endAt)} — {s.title}
              </li>
            ))}
          </ul>
        ) : null}
        {sampleTitles.length > 0 && (!testSlots || testSlots.length === 0) ? (
          <div className="mt-4 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
            <p className="font-semibold">Titres d’événements vus sur l’agenda (aperçu) :</p>
            <ul className="mt-1 list-disc pl-5">
              {sampleTitles.map((t) => (
                <li key={t}>
                  <code className="font-mono text-xs">{t}</code>
                </li>
              ))}
            </ul>
            <p className="mt-2 text-xs">
              Copiez un fragment dans « Texte recherché dans le titre Google » de cette
              direction, enregistrez, puis retestez.
            </p>
          </div>
        ) : null}
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
        <h2 className="text-lg font-bold text-slate-900">Liens publics à envoyer aux parents</h2>
        {publicLinks.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">
            Activez au moins une direction avec un calendarId pour obtenir un lien.
          </p>
        ) : (
          <ul className="mt-3 space-y-2">
            {publicLinks.map((l) => (
              <li
                key={l.slug}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-slate-100 bg-slate-50 px-3 py-2"
              >
                <div>
                  <p className="font-semibold text-slate-800">{l.label}</p>
                  <a
                    href={l.url}
                    target="_blank"
                    rel="noreferrer"
                    className="break-all text-xs text-sky-700 hover:underline"
                  >
                    {l.url}
                  </a>
                </div>
                <button
                  type="button"
                  onClick={() => void copyLink(l.url)}
                  className="inline-flex items-center gap-1 rounded-md border border-slate-300 px-2 py-1 text-xs font-semibold"
                >
                  {copied === l.url ? "Copié" : "Copier"}
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
        <h2 className="text-lg font-bold text-slate-900">Réservations récentes</h2>
        {bookings.length === 0 ? (
          <p className="mt-2 text-sm text-slate-500">Aucune réservation pour l’instant.</p>
        ) : (
          <div className="mt-3 overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead>
                <tr className="border-b text-slate-500">
                  <th className="py-2 pr-3 font-semibold">Créneau</th>
                  <th className="py-2 pr-3 font-semibold">Élève</th>
                  <th className="py-2 pr-3 font-semibold">Contact</th>
                  <th className="py-2 pr-3 font-semibold">Statut</th>
                  <th className="py-2 font-semibold">Agenda</th>
                </tr>
              </thead>
              <tbody>
                {bookings.map((b) => (
                  <tr key={b.id} className="border-b border-slate-100">
                    <td className="py-2 pr-3 whitespace-nowrap">
                      <span className="text-xs font-medium uppercase text-slate-500">
                        {b.directionSlug}
                      </span>
                      <br />
                      {formatSlot(b.startAt, b.endAt)}
                    </td>
                    <td className="py-2 pr-3">
                      {b.studentFirstName} {b.studentLastName}
                    </td>
                    <td className="py-2 pr-3">
                      <a className="text-sky-700 hover:underline" href={`mailto:${b.parentEmail}`}>
                        {b.parentEmail}
                      </a>
                      <br />
                      <span className="text-slate-600">{b.parentPhone}</span>
                    </td>
                    <td className="py-2 pr-3">
                      <span
                        className={`inline-flex rounded-full px-2 py-0.5 text-xs font-semibold ${
                          b.status === "confirmed"
                            ? "bg-emerald-100 text-emerald-800"
                            : b.status === "pending"
                              ? "bg-amber-100 text-amber-900"
                              : "bg-slate-100 text-slate-600"
                        }`}
                      >
                        {b.status === "confirmed"
                          ? "Confirmé"
                          : b.status === "pending"
                            ? "En attente mail"
                            : b.status === "expired"
                              ? "Expiré"
                              : "Annulé"}
                      </span>
                    </td>
                    <td className="py-2">
                      {b.googleHtmlLink ? (
                        <a
                          href={b.googleHtmlLink}
                          target="_blank"
                          rel="noreferrer"
                          className="inline-flex items-center gap-1 text-sky-700 hover:underline"
                        >
                          Ouvrir
                        </a>
                      ) : (
                        "—"
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}

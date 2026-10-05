"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { dash } from "@/app/lib/dashboard-brand";
import type {
  RdvInscriptionBookingRow,
  RdvInscriptionConfigPublic,
  RdvInscriptionDirectionRow,
  RdvInscriptionSlot,
} from "@/app/lib/rdv-inscription-types";
import { RDV_RESCHEDULE_PRESET_MOTIF } from "@/app/lib/rdv-inscription-types";

const FIELD = dash.field;
const FIELD_LABEL = dash.fieldLabel;

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

type AdminTab = "reglages" | "suivi";

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

function formatWhen(iso: string | null | undefined): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function normalizeBookingSearch(raw: string): string {
  return raw
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function bookingMatchesSearch(b: RdvInscriptionBookingRow, needle: string): boolean {
  if (!needle) return true;
  const haystack = normalizeBookingSearch(
    [
      b.studentFirstName,
      b.studentLastName,
      b.parentFirstName,
      b.parentLastName,
      b.parentEmail,
      b.parentPhone,
    ]
      .filter(Boolean)
      .join(" "),
  );
  return needle.split(/\s+/).filter(Boolean).every((token) => haystack.includes(token));
}

function readTabFromUrl(): AdminTab {
  if (typeof window === "undefined") return "suivi";
  const t = new URLSearchParams(window.location.search).get("tab");
  // Défaut = suivi (usage quotidien). `tab=suivi` reste accepté pour les anciens liens.
  return t === "reglages" ? "reglages" : "suivi";
}

function readDirectionFilterFromUrl(): string {
  if (typeof window === "undefined") return "";
  return (new URLSearchParams(window.location.search).get("direction") || "").trim();
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
  const [tab, setTab] = useState<AdminTab>("suivi");
  const [directionFilter, setDirectionFilter] = useState<string>("");
  const [bookingSearch, setBookingSearch] = useState("");
  const [rescheduleTarget, setRescheduleTarget] = useState<{
    id: string;
    studentLabel: string;
    slotLabel: string;
  } | null>(null);
  const [rescheduleNote, setRescheduleNote] = useState("");
  const [changeSlotTarget, setChangeSlotTarget] = useState<{
    id: string;
    studentLabel: string;
    slotLabel: string;
    currentEventId: string;
  } | null>(null);
  const [changeSlotNote, setChangeSlotNote] = useState("");
  const [changeSlotGoogleMode, setChangeSlotGoogleMode] = useState<"update" | "already_done">(
    "already_done",
  );
  const [changeSlotSelectedEventId, setChangeSlotSelectedEventId] = useState("");
  const [changeSlotOptions, setChangeSlotOptions] = useState<RdvInscriptionSlot[]>([]);
  const [changeSlotLoading, setChangeSlotLoading] = useState(false);

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
      setTab(readTabFromUrl());
      setDirectionFilter(readDirectionFilterFromUrl());
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

  function syncUrl(nextTab: AdminTab, nextDirection: string) {
    const url = new URL(window.location.href);
    // Suivi = onglet par défaut → pas de `tab` dans l’URL.
    if (nextTab === "suivi") url.searchParams.delete("tab");
    else url.searchParams.set("tab", nextTab);
    if (!nextDirection) url.searchParams.delete("direction");
    else url.searchParams.set("direction", nextDirection);
    window.history.replaceState({}, "", url.pathname + url.search);
  }

  function selectTab(next: AdminTab) {
    setTab(next);
    syncUrl(next, directionFilter);
  }

  function selectDirectionFilter(slug: string) {
    setDirectionFilter(slug);
    syncUrl(tab, slug);
  }

  const directionLabelBySlug = useMemo(() => {
    const map = new Map<string, string>();
    for (const d of data?.directions || []) {
      map.set(d.slug, d.label);
    }
    return map;
  }, [data?.directions]);

  const bookingSearchNeedle = useMemo(
    () => normalizeBookingSearch(bookingSearch),
    [bookingSearch],
  );

  const filteredBookings = useMemo(() => {
    return bookings.filter((b) => {
      if (directionFilter && b.directionSlug !== directionFilter) return false;
      return bookingMatchesSearch(b, bookingSearchNeedle);
    });
  }, [bookings, directionFilter, bookingSearchNeedle]);

  const bookingCountsByDirection = useMemo(() => {
    const counts = new Map<string, number>();
    for (const b of bookings) {
      counts.set(b.directionSlug, (counts.get(b.directionSlug) || 0) + 1);
    }
    return counts;
  }, [bookings]);

  const bookingStatusCounts = useMemo(() => {
    let pending = 0;
    let confirmed = 0;
    let other = 0;
    for (const b of bookings) {
      if (b.status === "pending") pending += 1;
      else if (b.status === "confirmed") confirmed += 1;
      else other += 1;
    }
    return { pending, confirmed, other, total: bookings.length };
  }, [bookings]);

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
      const json = (await res.json()) as {
        error?: string;
        success?: boolean;
        mailWarning?: string;
        already?: boolean;
        remainingCount?: number;
      };
      if (!res.ok) throw new Error(json.error || "Échec.");
      if (body.action === "confirm-booking") {
        setMessage(
          json.already
            ? "Réservation déjà confirmée."
            : json.mailWarning
              ? `Confirmé (attention mail : ${json.mailWarning})`
              : "Réservation confirmée — mail récap envoyé au parent.",
        );
      } else if (body.action === "cancel-booking") {
        const remaining = typeof json.remainingCount === "number" ? json.remainingCount : null;
        setMessage(
          json.mailWarning
            ? `Créneau supprimé (attention mail : ${json.mailWarning})`
            : remaining !== null
              ? `Créneau supprimé — mail envoyé au parent (${remaining} RDV restant${remaining > 1 ? "s" : ""}).`
              : "Créneau supprimé — mail envoyé au parent.",
        );
      } else if (body.action === "request-reschedule") {
        setMessage(
          json.mailWarning
            ? `Demande de rechoix envoyée (attention mail : ${json.mailWarning})`
            : "Créneau retiré — mail d’excuse avec lien de rechoix envoyé au parent.",
        );
        setRescheduleTarget(null);
        setRescheduleNote("");
      } else if (body.action === "resend-reschedule") {
        setMessage(
          json.mailWarning
            ? `Lien de rechoix renvoyé (attention mail : ${json.mailWarning})`
            : "Mail de rechoix renvoyé au parent (lien corrigé, valable 14 jours).",
        );
      } else if (body.action === "change-slot") {
        setMessage(
          json.mailWarning
            ? `Créneau modifié (attention mail : ${json.mailWarning})`
            : "Créneau modifié — mail + ICS envoyés au parent.",
        );
        setChangeSlotTarget(null);
        setChangeSlotNote("");
        setChangeSlotSelectedEventId("");
        setChangeSlotOptions([]);
      } else {
        setMessage("Enregistré.");
      }
      await load();
      return json;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }

  async function confirmPendingBooking(bookingId: string, studentLabel: string) {
    const ok = window.confirm(
      `Confirmer manuellement le rendez-vous de ${studentLabel} ?\n\nLe parent recevra le mail de confirmation avec fichier calendrier.`,
    );
    if (!ok) return;
    await put({ action: "confirm-booking", bookingId });
  }

  async function cancelBooking(
    bookingId: string,
    studentLabel: string,
    slotLabel: string,
  ) {
    const ok = window.confirm(
      `Supprimer le rendez-vous de ${studentLabel} ?\n${slotLabel}\n\nLe créneau sera remis libre dans Google Agenda, et le parent recevra un e-mail (avec rappel de ses autres RDV s’il en reste).`,
    );
    if (!ok) return;
    await put({ action: "cancel-booking", bookingId });
  }

  async function submitRescheduleRequest() {
    if (!rescheduleTarget) return;
    await put({
      action: "request-reschedule",
      bookingId: rescheduleTarget.id,
      note: rescheduleNote.trim() || undefined,
    });
  }

  async function resendRescheduleMail(
    bookingId: string,
    studentLabel: string,
  ) {
    const ok = window.confirm(
      `Renvoyer le mail de rechoix à ${studentLabel} ?\n\nLe parent recevra un nouveau lien (direction correcte, valable 14 jours). L’ancien lien ne fonctionnera plus.`,
    );
    if (!ok) return;
    await put({ action: "resend-reschedule", bookingId });
  }

  async function openChangeSlot(booking: RdvInscriptionBookingRow) {
    setChangeSlotTarget({
      id: booking.id,
      studentLabel: `${booking.studentFirstName} ${booking.studentLastName}`,
      slotLabel: formatSlot(booking.startAt, booking.endAt),
      currentEventId: booking.googleEventId,
    });
    setChangeSlotNote("");
    setChangeSlotGoogleMode("already_done");
    setChangeSlotSelectedEventId(booking.googleEventId);
    setChangeSlotOptions([]);
    setChangeSlotLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/rdv-inscription/admin", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "list-booking-slots", bookingId: booking.id }),
      });
      const json = (await res.json()) as {
        error?: string;
        slots?: RdvInscriptionSlot[];
        currentEventId?: string;
      };
      if (!res.ok) throw new Error(json.error || "Impossible de charger les créneaux.");
      setChangeSlotOptions(json.slots || []);
      if (json.currentEventId) {
        setChangeSlotSelectedEventId(json.currentEventId);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setChangeSlotLoading(false);
    }
  }

  async function submitChangeSlot() {
    if (!changeSlotTarget || !changeSlotSelectedEventId) return;
    await put({
      action: "change-slot",
      bookingId: changeSlotTarget.id,
      newEventId: changeSlotSelectedEventId,
      googleMode: changeSlotGoogleMode,
      note: changeSlotNote.trim() || undefined,
    });
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
    return (
      <ModulePageShell maxWidthClass="max-w-[90rem]">
        <p className={`text-sm ${dash.textMid}`}>Chargement…</p>
      </ModulePageShell>
    );
  }
  if (!data) {
    return (
      <ModulePageShell maxWidthClass="max-w-[90rem]">
        <ModuleCard bodyClassName="p-6">
          <p className="text-sm font-semibold text-red-700">{error || "Erreur"}</p>
          <ModuleButton className="mt-4" variant="secondary" onClick={() => void load()}>
            Réessayer
          </ModuleButton>
        </ModuleCard>
      </ModulePageShell>
    );
  }

  const { config, directions, google, publicLinks, oauthRedirectUri, oauthStartPath } = data;

  return (
    <ModulePageShell maxWidthClass="max-w-[90rem]">
      <div className="space-y-5">
      <ModulePageHeader
        eyebrow="Établissement"
        title="RDV inscription direction"
        description="Suivi des réservations parents et paramétrage des agendas (école, collège, lycée)."
        actions={
          tab === "suivi" ? (
            <ModuleButton
              variant="secondary"
              disabled={loading || busy}
              onClick={() => void load()}
            >
              Actualiser
            </ModuleButton>
          ) : null
        }
      />

      <ModuleTabNav
        className="mb-1"
        tabs={[
          { id: "suivi", label: "Suivi" },
          { id: "reglages", label: "Réglages" },
        ]}
        active={tab}
        onChange={(id) => selectTab(id)}
        badges={{ suivi: bookings.length }}
      />

      {message ? (
        <p className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
          {message}
        </p>
      ) : null}
      {error ? (
        <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
          {error}
        </p>
      ) : null}

      {tab === "reglages" ? (
        <div className="grid gap-4 lg:grid-cols-12">
          <ModuleCard className="lg:col-span-5" bodyClassName="flex h-full flex-col gap-4 p-5 sm:p-6">
            <div>
              <p className={FIELD_LABEL}>Compte Google</p>
              <h2 className={`mt-1 text-lg font-semibold tracking-tight ${dash.ink}`}>
                Compte technique
              </h2>
              <p className={`mt-1 text-sm ${dash.textMid}`}>
                Connectez un compte Google, puis demandez à chaque directrice de{" "}
                <strong>partager son agenda</strong> avec ce compte (droits de modification des
                événements).
              </p>
            </div>
            {!google.clientConfigured ? (
              <p className="rounded-2xl bg-amber-50 px-3 py-2 text-sm text-amber-900">
                Configurez <code className="font-mono text-xs">GOOGLE_CLIENT_ID</code> et{" "}
                <code className="font-mono text-xs">GOOGLE_CLIENT_SECRET</code> (ou secrets.google du
                tenant). URI de redirection à enregistrer dans Google Cloud :{" "}
                <code className="break-all font-mono text-xs">
                  {oauthRedirectUri || "…/api/rdv-inscription/oauth/callback"}
                </code>
              </p>
            ) : null}
            <div className="flex flex-wrap items-center gap-2">
              {google.linked ? (
                <>
                  <span className="inline-flex items-center rounded-full bg-emerald-100 px-3 py-1 text-sm font-semibold text-emerald-900">
                    Lié
                    {google.linkedEmail ? ` — ${google.linkedEmail}` : ""}
                  </span>
                  <ModuleButton
                    variant="secondary"
                    disabled={busy}
                    onClick={() => void put({ action: "unlink-google" })}
                  >
                    Déconnecter
                  </ModuleButton>
                  <a
                    href={oauthStartPath}
                    className={`inline-flex items-center rounded-xl border bg-white px-4 py-2.5 text-sm font-bold ${dash.border} ${dash.ink} ${dash.hoverBorder}`}
                  >
                    Reconnecter
                  </a>
                </>
              ) : (
                <>
                  <a
                    href={oauthStartPath}
                    className={`inline-flex items-center rounded-xl px-4 py-2.5 text-sm ${dash.btnPrimary}`}
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
              className="mt-auto space-y-3 border-t border-black/6 pt-4"
              onSubmit={(e) => {
                e.preventDefault();
                const fd = new FormData(e.currentTarget);
                void put({
                  action: "save-config",
                  config: { enabled: fd.get("enabled") === "on" },
                });
              }}
            >
              <label className={`flex items-center gap-2 text-sm ${dash.ink}`}>
                <input type="checkbox" name="enabled" defaultChecked={config.enabled} />
                <span className="font-semibold">
                  Coupe-circuit global (désactiver toutes les pages)
                </span>
              </label>
              <ModuleButton type="submit" variant="secondary" disabled={busy}>
                Enregistrer
              </ModuleButton>
            </form>
          </ModuleCard>

          <ModuleCard className="lg:col-span-7" bodyClassName="flex h-full flex-col gap-4 p-5 sm:p-6">
            <div>
              <p className={FIELD_LABEL}>Parents</p>
              <h2 className={`mt-1 text-lg font-semibold tracking-tight ${dash.ink}`}>
                Liens publics
              </h2>
              <p className={`mt-1 text-sm ${dash.textMid}`}>
                À envoyer aux familles pour prendre rendez-vous avec chaque direction.
              </p>
            </div>
            {publicLinks.length === 0 ? (
              <p className={`text-sm ${dash.textMid}`}>
                Activez au moins une direction avec un calendarId pour obtenir un lien.
              </p>
            ) : (
              <ul className="grid gap-2 sm:grid-cols-2">
                {publicLinks.map((l) => (
                  <li
                    key={l.slug}
                    className="flex flex-col justify-between gap-3 rounded-2xl border border-black/6 bg-[color:var(--dash-soft-muted)]/40 px-4 py-3"
                  >
                    <div className="min-w-0">
                      <p className={`font-semibold ${dash.ink}`}>{l.label}</p>
                      <a
                        href={l.url}
                        target="_blank"
                        rel="noreferrer"
                        className="mt-1 block break-all text-xs text-[var(--dash-primary)] hover:underline"
                      >
                        {l.url}
                      </a>
                    </div>
                    <ModuleButton
                      variant="secondary"
                      className="self-start !px-3 !py-1.5 !text-xs"
                      onClick={() => void copyLink(l.url)}
                    >
                      {copied === l.url ? "Copié" : "Copier"}
                    </ModuleButton>
                  </li>
                ))}
              </ul>
            )}
          </ModuleCard>

          <div className="space-y-3 lg:col-span-12">
            <div>
              <p className={FIELD_LABEL}>Configuration</p>
              <h2 className={`mt-1 text-lg font-semibold tracking-tight ${dash.ink}`}>
                Directions & agendas
              </h2>
              <p className={`mt-1 text-sm ${dash.textMid}`}>
                Paramétrez chaque direction indépendamment (agenda, textes, motif Google, notif
                secrétariat, horizon).
              </p>
            </div>
            <div className="grid gap-4 xl:grid-cols-2">
              {directions.map((d) => (
                <ModuleCard
                  key={`${d.id}-${d.eventTitlePattern}-${d.notifyEmail || ""}-${d.horizonDays}-${d.title}`}
                  bodyClassName="p-5 sm:p-6"
                >
                  <form
                    className="grid gap-3 sm:grid-cols-2"
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
                          directriceDisplayName:
                            String(fd.get("directriceDisplayName") || "") || null,
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
                    <div className="sm:col-span-2 flex flex-wrap items-center justify-between gap-2 border-b border-black/6 pb-3">
                      <p className={`text-base font-semibold ${dash.ink}`}>{d.label}</p>
                      <span
                        className={`rounded-full px-2.5 py-0.5 text-[11px] font-bold ${
                          d.active
                            ? "bg-emerald-100 text-emerald-800"
                            : "bg-slate-100 text-slate-600"
                        }`}
                      >
                        {d.active ? "Active" : "Inactive"}
                      </span>
                    </div>
                    <label className="block">
                      <span className={FIELD_LABEL}>Libellé</span>
                      <input name="label" defaultValue={d.label} className={`mt-1.5 ${FIELD}`} />
                    </label>
                    <label className="block">
                      <span className={FIELD_LABEL}>Slug URL</span>
                      <input
                        name="slug"
                        defaultValue={d.slug}
                        className={`mt-1.5 font-mono text-xs ${FIELD}`}
                      />
                    </label>
                    <label className="block sm:col-span-2">
                      <span className={FIELD_LABEL}>Google Calendar ID</span>
                      <input
                        name="googleCalendarId"
                        defaultValue={d.googleCalendarId}
                        placeholder="directrice@ecole.fr"
                        className={`mt-1.5 font-mono text-xs ${FIELD}`}
                      />
                    </label>
                    <label className="block">
                      <span className={FIELD_LABEL}>Nom directrice (affiché)</span>
                      <input
                        name="directriceDisplayName"
                        defaultValue={d.directriceDisplayName || ""}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className="block">
                      <span className={FIELD_LABEL}>Ordre</span>
                      <input
                        name="sortOrder"
                        type="number"
                        defaultValue={d.sortOrder}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className="block sm:col-span-2">
                      <span className={FIELD_LABEL}>Titre page publique</span>
                      <input name="title" defaultValue={d.title} className={`mt-1.5 ${FIELD}`} />
                    </label>
                    <label className="block sm:col-span-2">
                      <span className={FIELD_LABEL}>Introduction</span>
                      <textarea
                        name="intro"
                        rows={2}
                        defaultValue={d.intro}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className="block sm:col-span-2">
                      <span className={FIELD_LABEL}>Texte recherché dans le titre Google</span>
                      <input
                        name="eventTitlePattern"
                        defaultValue={d.eventTitlePattern}
                        placeholder="RDV inscription"
                        className={`mt-1.5 ${FIELD}`}
                      />
                      <span className={`mt-1 block text-xs ${dash.textMid}`}>
                        Exactement ce que cette directrice écrit dans le titre (casse/accents
                        ignorés). Plusieurs formulations :{" "}
                        <code className="font-mono">
                          RDV inscription | rendez-vous inscription
                        </code>
                        .
                      </span>
                    </label>
                    <label className="block">
                      <span className={FIELD_LABEL}>Horizon (jours)</span>
                      <input
                        name="horizonDays"
                        type="number"
                        min={7}
                        max={180}
                        defaultValue={d.horizonDays}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className="block">
                      <span className={FIELD_LABEL}>E-mail notif secrétariat</span>
                      <input
                        name="notifyEmail"
                        type="email"
                        defaultValue={d.notifyEmail || ""}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className="block">
                      <span className={FIELD_LABEL}>Lieu</span>
                      <input
                        name="location"
                        defaultValue={d.location}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className="block sm:col-span-2">
                      <span className={FIELD_LABEL}>Libellé consentement</span>
                      <input
                        name="consentLabel"
                        defaultValue={d.consentLabel}
                        className={`mt-1.5 ${FIELD}`}
                      />
                    </label>
                    <label className={`flex items-center gap-2 text-sm sm:col-span-2 ${dash.ink}`}>
                      <input type="checkbox" name="active" defaultChecked={d.active} />
                      <span className="font-semibold">Direction active (page publique)</span>
                    </label>
                    <div className="flex flex-wrap gap-2 sm:col-span-2">
                      <ModuleButton type="submit" disabled={busy}>
                        Enregistrer
                      </ModuleButton>
                      <ModuleButton
                        type="button"
                        variant="secondary"
                        disabled={busy || !google.linked || !d.googleCalendarId}
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
                      </ModuleButton>
                    </div>
                  </form>
                </ModuleCard>
              ))}
            </div>
            {testSlots && testSlots.length > 0 ? (
              <ModuleCard bodyClassName="p-4 sm:p-5">
                <p className={`text-sm font-semibold ${dash.ink}`}>Créneaux libres (aperçu)</p>
                <ul className={`mt-2 space-y-1 text-sm ${dash.textMid}`}>
                  {testSlots.map((s) => (
                    <li key={s.eventId}>
                      {formatSlot(s.startAt, s.endAt)} — {s.title}
                    </li>
                  ))}
                </ul>
              </ModuleCard>
            ) : null}
            {sampleTitles.length > 0 && (!testSlots || testSlots.length === 0) ? (
              <ModuleCard
                className="border-amber-200/80 bg-amber-50/70"
                bodyClassName="p-4 sm:p-5 text-sm text-amber-950"
              >
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
              </ModuleCard>
            ) : null}
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-3">
            <ModuleCard bodyClassName="p-4 sm:p-5">
              <p className={FIELD_LABEL}>Total</p>
              <p className={`mt-2 text-3xl font-semibold tracking-tight ${dash.ink}`}>
                {bookingStatusCounts.total}
              </p>
              <p className={`mt-1 text-xs ${dash.textMid}`}>rendez-vous listés</p>
            </ModuleCard>
            <ModuleCard bodyClassName="p-4 sm:p-5">
              <p className={FIELD_LABEL}>Confirmés</p>
              <p className="mt-2 text-3xl font-semibold tracking-tight text-emerald-700">
                {bookingStatusCounts.confirmed}
              </p>
              <p className={`mt-1 text-xs ${dash.textMid}`}>prêts pour la direction</p>
            </ModuleCard>
            <ModuleCard bodyClassName="p-4 sm:p-5">
              <p className={FIELD_LABEL}>En attente</p>
              <p className="mt-2 text-3xl font-semibold tracking-tight text-amber-700">
                {bookingStatusCounts.pending}
              </p>
              <p className={`mt-1 text-xs ${dash.textMid}`}>
                + {bookingStatusCounts.other} annulé / expiré
              </p>
            </ModuleCard>
          </div>

          <ModuleCard bodyClassName="p-5 sm:p-6">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 className={`text-lg font-semibold tracking-tight ${dash.ink}`}>
                  Suivi des rendez-vous
                </h2>
                <p className={`mt-1 text-sm ${dash.textMid}`}>
                  Triés par date de prise (plus récent en haut). Les créneaux remplacés
                  apparaissent en « Annulé ».
                </p>
              </div>
              <p className={`text-sm font-semibold tabular-nums ${dash.textMid}`}>
                {filteredBookings.length}
                <span className="font-normal"> affiché{filteredBookings.length > 1 ? "s" : ""}</span>
              </p>
            </div>

            <div className="mt-5 space-y-3 border-b border-black/6 pb-4">
              <label className="block">
                <span className="sr-only">Rechercher un rendez-vous</span>
                <input
                  type="search"
                  value={bookingSearch}
                  onChange={(e) => setBookingSearch(e.target.value)}
                  placeholder="Rechercher par nom, prénom (élève ou contact)…"
                  autoComplete="off"
                  className={FIELD}
                />
              </label>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  onClick={() => selectDirectionFilter("")}
                  className={`rounded-full px-3 py-1.5 text-xs font-bold transition ${
                    !directionFilter
                      ? "bg-[var(--dash-ink)] text-white"
                      : `border border-black/8 bg-white ${dash.ink} ${dash.hoverBorder}`
                  }`}
                >
                  Tous
                  <span className="ml-1.5 tabular-nums opacity-80">{bookings.length}</span>
                </button>
                {directions.map((d) => {
                  const count = bookingCountsByDirection.get(d.slug) || 0;
                  const active = directionFilter === d.slug;
                  return (
                    <button
                      key={d.id}
                      type="button"
                      onClick={() => selectDirectionFilter(d.slug)}
                      className={`rounded-full px-3 py-1.5 text-xs font-bold transition ${
                        active
                          ? "bg-[var(--dash-primary)] text-white"
                          : `border border-black/8 bg-white ${dash.ink} ${dash.hoverBorder}`
                      }`}
                    >
                      {d.label}
                      <span className="ml-1.5 tabular-nums opacity-80">{count}</span>
                    </button>
                  );
                })}
              </div>
            </div>

            {filteredBookings.length === 0 ? (
              <p className={`mt-6 text-sm ${dash.textMid}`}>
                {bookings.length === 0
                  ? "Aucune réservation pour l’instant."
                  : bookingSearchNeedle
                    ? "Aucun rendez-vous ne correspond à cette recherche."
                    : "Aucune réservation pour ce filtre."}
              </p>
            ) : (
              <div className="mt-4 -mx-1 overflow-x-auto">
                <table className="min-w-full text-left text-sm">
                  <thead>
                    <tr className={`border-b border-black/6 text-xs uppercase tracking-[0.14em] ${dash.textMid}`}>
                      <th className="py-3 pr-4 font-semibold">Réservé le</th>
                      <th className="py-3 pr-4 font-semibold">Créneau</th>
                      <th className="py-3 pr-4 font-semibold">Élève</th>
                      <th className="py-3 pr-4 font-semibold">Contact</th>
                      <th className="py-3 pr-4 font-semibold">Statut</th>
                      <th className="py-3 font-semibold">Agenda</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredBookings.map((b) => {
                      const dirLabel =
                        directionLabelBySlug.get(b.directionSlug) || b.directionSlug;
                      return (
                        <tr
                          key={b.id}
                          className="border-b border-black/5 align-top last:border-b-0"
                        >
                          <td className={`py-3.5 pr-4 whitespace-nowrap text-xs ${dash.textMid}`}>
                            {formatWhen(b.createdAt)}
                            {b.confirmedAt ? (
                              <>
                                <br />
                                <span className="text-emerald-700">
                                  Conf. {formatWhen(b.confirmedAt)}
                                </span>
                              </>
                            ) : null}
                          </td>
                          <td className="py-3.5 pr-4 whitespace-nowrap">
                            <span className="text-xs font-semibold text-[var(--dash-primary)]">
                              {dirLabel}
                            </span>
                            <br />
                            <span className={dash.ink}>{formatSlot(b.startAt, b.endAt)}</span>
                          </td>
                          <td className={`py-3.5 pr-4 ${dash.ink}`}>
                            <span className="font-semibold">
                              {b.studentFirstName} {b.studentLastName}
                            </span>
                            {b.niveauLabel ? (
                              <>
                                <br />
                                <span className={`text-xs ${dash.textMid}`}>
                                  Niveau demandé : {b.niveauLabel}
                                  {b.regime ? ` · ${b.regime}` : ""}
                                </span>
                              </>
                            ) : b.regime ? (
                              <>
                                <br />
                                <span className={`text-xs ${dash.textMid}`}>
                                  Régime : {b.regime}
                                </span>
                              </>
                            ) : null}
                            {b.etablissementOrigineLabel ? (
                              <>
                                <br />
                                <span className={`text-xs ${dash.textMid}`}>
                                  Origine : {b.etablissementOrigineLabel}
                                </span>
                              </>
                            ) : null}
                            {b.hasPap === "yes" ? (
                              <>
                                <br />
                                <span className="text-xs text-amber-800">
                                  PAP : oui
                                  {b.papS3Key
                                    ? " (déposé)"
                                    : b.papBringToRdv
                                      ? " — à apporter"
                                      : ""}
                                </span>
                              </>
                            ) : b.hasPap === "no" ? (
                              <>
                                <br />
                                <span className={`text-xs ${dash.textMid}`}>PAP : non</span>
                              </>
                            ) : null}
                          </td>
                          <td className="py-3.5 pr-4">
                            {[b.parentFirstName, b.parentLastName].filter(Boolean).join(" ") || (
                              <span className={dash.textMid}>—</span>
                            )}
                            {b.rdvAttendee ? (
                              <>
                                <br />
                                <span className={`text-xs ${dash.textMid}`}>
                                  Présent :{" "}
                                  {b.rdvAttendee === "madame"
                                    ? "Madame"
                                    : b.rdvAttendee === "monsieur"
                                      ? "Monsieur"
                                      : "Les deux"}
                                </span>
                              </>
                            ) : null}
                            <br />
                            <a
                              className="text-[var(--dash-primary)] hover:underline"
                              href={`mailto:${b.parentEmail}`}
                            >
                              {b.parentEmail}
                            </a>
                            <br />
                            <span className={dash.textMid}>{b.parentPhone}</span>
                          </td>
                          <td className="py-3.5 pr-4">
                            <span
                              className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-semibold ${
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
                            {b.status === "pending" ? (
                              <button
                                type="button"
                                disabled={busy}
                                onClick={() =>
                                  void confirmPendingBooking(
                                    b.id,
                                    `${b.studentFirstName} ${b.studentLastName}`,
                                  )
                                }
                                className="mt-1.5 block text-xs font-semibold text-[var(--dash-primary)] hover:underline disabled:opacity-50"
                              >
                                Confirmer maintenant
                              </button>
                            ) : null}
                            {b.status === "pending" || b.status === "confirmed" ? (
                              <>
                                <button
                                  type="button"
                                  disabled={busy || changeSlotLoading}
                                  onClick={() => void openChangeSlot(b)}
                                  className="mt-1.5 block text-xs font-semibold text-[var(--dash-primary)] hover:underline disabled:opacity-50"
                                >
                                  Modifier le créneau
                                </button>
                                <button
                                  type="button"
                                  disabled={busy}
                                  onClick={() => {
                                    setRescheduleNote("");
                                    setRescheduleTarget({
                                      id: b.id,
                                      studentLabel: `${b.studentFirstName} ${b.studentLastName}`,
                                      slotLabel: formatSlot(b.startAt, b.endAt),
                                    });
                                  }}
                                  className="mt-1.5 block text-xs font-semibold text-amber-800 hover:underline disabled:opacity-50"
                                >
                                  Demander un autre créneau
                                </button>
                                <button
                                  type="button"
                                  disabled={busy}
                                  onClick={() =>
                                    void cancelBooking(
                                      b.id,
                                      `${b.studentFirstName} ${b.studentLastName}`,
                                      formatSlot(b.startAt, b.endAt),
                                    )
                                  }
                                  className="mt-1.5 block text-xs font-semibold text-red-700 hover:underline disabled:opacity-50"
                                >
                                  Supprimer
                                </button>
                              </>
                            ) : null}
                            {b.rescheduleLinkAvailable ? (
                              <button
                                type="button"
                                disabled={busy}
                                onClick={() =>
                                  void resendRescheduleMail(
                                    b.id,
                                    `${b.studentFirstName} ${b.studentLastName}`,
                                  )
                                }
                                className="mt-1.5 block text-xs font-semibold text-amber-800 hover:underline disabled:opacity-50"
                              >
                                Renvoyer le lien de rechoix
                              </button>
                            ) : null}
                          </td>
                          <td className="py-3.5">
                            {b.googleHtmlLink ? (
                              <a
                                href={b.googleHtmlLink}
                                target="_blank"
                                rel="noreferrer"
                                className="inline-flex items-center gap-1 font-semibold text-[var(--dash-primary)] hover:underline"
                              >
                                Ouvrir
                              </a>
                            ) : (
                              <span className={dash.textMid}>—</span>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </ModuleCard>
        </div>
      )}

      {rescheduleTarget ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4"
          role="dialog"
          aria-modal="true"
          aria-labelledby="reschedule-title"
          onClick={() => {
            if (!busy) {
              setRescheduleTarget(null);
              setRescheduleNote("");
            }
          }}
        >
          <div
            className="w-full max-w-lg rounded-[1.75rem] border border-black/6 bg-white p-5 shadow-xl sm:p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 id="reschedule-title" className="text-lg font-bold text-slate-900">
              Demander un autre créneau
            </h3>
            <p className="mt-1 text-sm text-slate-600">
              {rescheduleTarget.studentLabel} — {rescheduleTarget.slotLabel}
            </p>
            <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
              {RDV_RESCHEDULE_PRESET_MOTIF}
            </p>
            <p className="mt-3 text-xs text-slate-500">
              Le créneau sera <strong>retiré</strong> de Google Agenda (il ne sera plus libre). Le
              parent recevra un mail d’excuse avec un lien pour en choisir un autre.
            </p>
            <label className="mt-4 block text-sm">
              <span className="font-semibold text-slate-800">
                Précision optionnelle (ajoutée au mail)
              </span>
              <textarea
                rows={3}
                value={rescheduleNote}
                onChange={(e) => setRescheduleNote(e.target.value)}
                maxLength={1000}
                placeholder="Ex. indisponibilité imprévue, réunion urgente…"
                className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-sm"
              />
            </label>
            <div className="mt-5 flex flex-wrap justify-end gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setRescheduleTarget(null);
                  setRescheduleNote("");
                }}
                className="rounded-md border border-slate-300 px-3 py-1.5 text-sm font-semibold text-slate-700 disabled:opacity-50"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void submitRescheduleRequest()}
                className="rounded-md bg-amber-700 px-3 py-1.5 text-sm font-bold text-white hover:bg-amber-800 disabled:opacity-50"
              >
                {busy ? "Envoi…" : "Retirer et prévenir le parent"}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {changeSlotTarget ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4"
          role="dialog"
          aria-modal="true"
          aria-labelledby="change-slot-title"
          onClick={() => {
            if (!busy && !changeSlotLoading) {
              setChangeSlotTarget(null);
              setChangeSlotNote("");
              setChangeSlotOptions([]);
            }
          }}
        >
          <div
            className="max-h-[90vh] w-full max-w-xl overflow-y-auto rounded-[1.75rem] border border-black/6 bg-white p-5 shadow-xl sm:p-6"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 id="change-slot-title" className="text-lg font-bold text-slate-900">
              Modifier le créneau
            </h3>
            <p className="mt-1 text-sm text-slate-600">
              {changeSlotTarget.studentLabel} — actuel : {changeSlotTarget.slotLabel}
            </p>
            <p className="mt-3 text-sm text-slate-600">
              Après un appel téléphone : choisissez le nouveau créneau (ou resynchronisez si vous
              avez déjà déplacé l’événement dans Google Agenda). Le parent reçoit un mail avec le
              nouvel horaire et un fichier calendrier (.ics).
            </p>

            <fieldset className="mt-4 space-y-2 rounded-xl border border-slate-200 bg-slate-50 p-3">
              <legend className="px-1 text-sm font-semibold text-slate-800">
                Google Agenda
              </legend>
              <label className="flex cursor-pointer items-start gap-2 text-sm text-slate-800">
                <input
                  type="radio"
                  className="mt-1"
                  name="change-slot-google"
                  checked={changeSlotGoogleMode === "already_done"}
                  onChange={() => setChangeSlotGoogleMode("already_done")}
                />
                <span>
                  <strong>Déjà fait sur Google Agenda</strong>
                  <span className="mt-0.5 block text-xs text-slate-500">
                    Vous avez déjà déplacé / modifié l’événement. On met à jour ScolIA et on
                    prévient le parent, sans toucher à l’ancien créneau libre.
                  </span>
                </span>
              </label>
              <label className="flex cursor-pointer items-start gap-2 text-sm text-slate-800">
                <input
                  type="radio"
                  className="mt-1"
                  name="change-slot-google"
                  checked={changeSlotGoogleMode === "update"}
                  onChange={() => setChangeSlotGoogleMode("update")}
                />
                <span>
                  <strong>Mettre aussi à jour Google Agenda</strong>
                  <span className="mt-0.5 block text-xs text-slate-500">
                    Remet l’ancien créneau libre et réserve le nouveau (titre élève + détails).
                  </span>
                </span>
              </label>
            </fieldset>

            <div className="mt-4">
              <p className="text-sm font-semibold text-slate-800">Nouveau créneau</p>
              {changeSlotLoading ? (
                <p className="mt-2 text-sm text-slate-500">Chargement des créneaux libres…</p>
              ) : (
                <ul className="mt-2 max-h-56 space-y-1 overflow-y-auto">
                  <li>
                    <button
                      type="button"
                      onClick={() =>
                        setChangeSlotSelectedEventId(changeSlotTarget.currentEventId)
                      }
                      className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                        changeSlotSelectedEventId === changeSlotTarget.currentEventId
                          ? "bg-sky-700 text-white"
                          : "bg-emerald-50 text-emerald-950 ring-1 ring-emerald-200 hover:bg-emerald-100"
                      }`}
                    >
                      <span className="font-semibold">
                        Resynchroniser l’événement Google actuel
                      </span>
                      <span
                        className={`mt-0.5 block text-xs ${
                          changeSlotSelectedEventId === changeSlotTarget.currentEventId
                            ? "text-sky-100"
                            : "text-emerald-800/80"
                        }`}
                      >
                        Si vous avez déjà déplacé le même rendez-vous dans Agenda
                      </span>
                    </button>
                  </li>
                  {changeSlotOptions.map((s) => {
                    const selected = changeSlotSelectedEventId === s.eventId;
                    return (
                      <li key={s.eventId}>
                        <button
                          type="button"
                          onClick={() => setChangeSlotSelectedEventId(s.eventId)}
                          className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                            selected
                              ? "bg-sky-700 text-white"
                              : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white"
                          }`}
                        >
                          <span className="font-semibold">
                            {formatSlot(s.startAt, s.endAt)}
                          </span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
              {!changeSlotLoading && changeSlotOptions.length === 0 ? (
                <p className="mt-2 text-xs text-slate-500">
                  Aucun autre créneau libre dans l’horizon — utilisez la resynchronisation si
                  l’horaire a déjà changé dans Agenda.
                </p>
              ) : null}
            </div>

            <label className="mt-4 block text-sm">
              <span className="font-semibold text-slate-800">
                Précision optionnelle (ajoutée au mail)
              </span>
              <textarea
                rows={2}
                value={changeSlotNote}
                onChange={(e) => setChangeSlotNote(e.target.value)}
                maxLength={1000}
                placeholder="Ex. créneau décalé suite à notre échange téléphonique…"
                className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2 text-sm"
              />
            </label>

            <div className="mt-5 flex flex-wrap justify-end gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setChangeSlotTarget(null);
                  setChangeSlotNote("");
                  setChangeSlotOptions([]);
                }}
                className="rounded-md border border-slate-300 px-3 py-1.5 text-sm font-semibold text-slate-700 disabled:opacity-50"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={busy || changeSlotLoading || !changeSlotSelectedEventId}
                onClick={() => void submitChangeSlot()}
                className="rounded-md bg-sky-700 px-3 py-1.5 text-sm font-bold text-white hover:bg-sky-800 disabled:opacity-50"
              >
                {busy ? "Enregistrement…" : "Enregistrer et prévenir le parent"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
      </div>
    </ModulePageShell>
  );
}

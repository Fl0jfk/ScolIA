/**
 * « Fil du voyage » — règles PURES (aucun accès base, stockage ni réseau) : droits, état du fil, publications,
 * droit à l'image, réactions, regroupement des e-mails, durée de vie.
 * Données d'enfants : en cas de doute, la règle la plus restrictive l'emporte.
 */

import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { isTripOrganizerStrict, resolveContactsAccess } from "@/app/lib/travels-parent-contacts";
import { isGlobalAdminUser } from "@/app/lib/travels-roles";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";
import { addDaysIso, isIsoDate, tripDepartureIso, tripReturnIso } from "@/app/lib/travels-v2";

/* ─── Constantes ─── */

export const FEED_KINDS = ["arrives", "ok", "delay", "departure_back", "at_school", "program", "message", "photos"] as const;
export type FeedKind = (typeof FEED_KINDS)[number];

export const FEED_KIND_LABELS: Record<FeedKind, string> = {
  arrives: "Bien arrivés",
  ok: "Tout va bien",
  delay: "Petit retard",
  departure_back: "Départ du retour",
  at_school: "Arrivés à l'établissement",
  program: "Programme du jour",
  message: "Message",
  photos: "Photos",
};

/** Réactions autorisées (une par type et par famille). */
export const REACTION_EMOJIS = ["👍", "❤️", "😄", "👏"] as const;
export type ReactionEmoji = (typeof REACTION_EMOJIS)[number];

export const MESSAGE_MAX_CHARS = 500;
export const PROGRAM_MAX_CHARS = 1500;
export const PHOTOS_MAX_PER_POST = 10;
export const DELAY_MIN_MINUTES = 5;
export const DELAY_MAX_MINUTES = 600;

/** Suppression de tout le fil (textes, photos, réactions, accès) 7 jours après le retour. */
export const FEED_PURGE_DAYS = 7;
/** Durée de vie d'une URL pré-signée de photo (secondes) — générée à chaque affichage, jamais stockée. */
export const SIGNED_PHOTO_TTL_SECONDS = 600;
/** Au plus un e-mail « nouveautés » par famille et par heure. */
export const FEED_EMAIL_MIN_INTERVAL_MS = 60 * 60_000;
/** Plafonds d'envoi : par voyage (tous les e-mails de nouveautés) et par famille. */
export const FEED_EMAIL_CAP_PER_TRIP = 1500;
export const FEED_EMAIL_CAP_PER_FAMILY = 30;
/** Nombre de liens personnels valides conservés par adresse (chaque e-mail en émet un nouveau). */
export const FEED_MAX_TOKEN_HASHES = 40;

/* ─── Participants et état du fil ─── */

/** Participants du fil = liste OFFICIELLE (`participantEleves`) : inscrits, ni désistés ni retirés. Jamais la pré-liste. */
export function feedParticipants(trip: Pick<TravelsTrip, "data">): TravelsParticipantEleve[] {
  return (trip.data?.participantEleves ?? []).filter((p) => p && String(p.ine ?? "").trim());
}

const keyOfIne = (ine: string) => String(ine || "").trim().toUpperCase();

/** INE de l'accès familial qui sont TOUJOURS participants (un élève retiré après l'ouverture ne donne plus accès). */
export function stillParticipantInes(accessInes: string[], participants: Pick<TravelsParticipantEleve, "ine">[]): string[] {
  const official = new Set(participants.map((p) => keyOfIne(p.ine)));
  return accessInes.filter((i) => official.has(keyOfIne(i)));
}

/** Clé de famille : premier INE (trié) — deux responsables d'un même enfant partagent leurs réactions. */
export function familyKeyOf(ines: string[]): string {
  const sorted = [...new Set(ines.map(keyOfIne).filter(Boolean))].sort();
  return sorted[0] || "";
}

const CLOSED_STATUSES = new Set(["ANNULE", "REJETE", "SEANCE_ANNULEE"]);

/** Le fil ne s'ouvre que sur un voyage finalisé (liste officielle figée) et non annulé. */
export function canOpenFeedOnStatus(status: string | null | undefined): boolean {
  return status === "VALIDE" || status === "FINALISE_DIR_ATTENTE_ELEVES";
}

/** Date de retour effective (AAAA-MM-JJ) : retour, à défaut départ. */
export function feedReturnIso(trip: Pick<TravelsTrip, "data">): string | null {
  return tripReturnIso(trip.data) ?? tripDepartureIso(trip.data);
}

export type FeedPhase = "unavailable" | "live" | "readonly" | "purge_due";

/**
 * - `live` : publication possible jusqu'au jour du retour inclus ;
 * - `readonly` : après la date de retour, lecture seule ;
 * - `purge_due` : retour + 7 jours atteint, tout doit être supprimé (et plus rien n'est servi) ;
 * - `unavailable` : voyage annulé / refusé, ou sans date de retour (jamais de fil sans échéance de purge).
 */
export function feedPhase(opts: { status: string | null | undefined; returnIso: string | null | undefined; todayIso: string }): FeedPhase {
  if (CLOSED_STATUSES.has(String(opts.status))) return "unavailable";
  if (!isIsoDate(opts.returnIso)) return "unavailable";
  if (opts.todayIso >= addDaysIso(opts.returnIso, FEED_PURGE_DAYS)) return "purge_due";
  if (opts.todayIso > opts.returnIso) return "readonly";
  return "live";
}

export function feedPurgeDay(returnIso: string): string {
  return addDaysIso(returnIso, FEED_PURGE_DAYS);
}

/** Purge due : retour + 7 jours atteint, ou voyage annulé / refusé (le fil n'a plus d'objet). */
export function isFeedPurgeDue(opts: { status: string | null | undefined; returnIso: string | null | undefined; todayIso: string }): boolean {
  if (CLOSED_STATUSES.has(String(opts.status))) return true;
  if (!isIsoDate(opts.returnIso)) return false;
  return opts.todayIso >= feedPurgeDay(opts.returnIso);
}

/* ─── Droits des profs ─── */

export type FeedStaffRights = { canPublish: boolean; canModerate: boolean; reason: "owner" | "escort" | "admin" | "direction" | "forbidden" };

/**
 * Publier : organisateur, accompagnateurs de CE voyage (id ou e-mail de compte, jamais le nom), admin général.
 * Modérer / ouvrir le fil / régler : organisateur, direction de l'établissement, admin général.
 */
export function resolveFeedStaffRights(
  trip: Pick<TravelsTrip, "status" | "data" | "ownerId" | "ownerName"> & { ownerEmail?: string | null },
  user: SessionLikeUser | null | undefined,
  isDirection: boolean,
): FeedStaffRights {
  if (!user) return { canPublish: false, canModerate: false, reason: "forbidden" };
  const isAdmin = isGlobalAdminUser({ publicMetadata: user.publicMetadata ?? null });
  const contacts = resolveContactsAccess(trip, user);
  const owner = isTripOrganizerStrict(trip, user);
  const publish = (contacts.allowed && contacts.reason !== "cancelled") || isAdmin;
  const moderate = isAdmin || owner || (isDirection && !CLOSED_STATUSES.has(String(trip.status)));
  const reason: FeedStaffRights["reason"] = isAdmin ? "admin" : owner ? "owner" : contacts.allowed ? "escort" : moderate ? "direction" : "forbidden";
  return { canPublish: publish, canModerate: moderate, reason };
}

/* ─── Publications ─── */

export type FeedPostInput =
  | { ok: true; kind: FeedKind; body: string | null; delayMinutes: number | null }
  | { ok: false; error: string };

function cleanText(raw: unknown, max: number): string {
  return String(raw ?? "")
    .replace(/\r\n?/g, "\n")
    // eslint-disable-next-line no-control-regex
    .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/g, "")
    .replace(/\n{3,}/g, "\n\n")
    .trim()
    .slice(0, max);
}

/** Valide une publication texte (raccourci, programme ou message). Les photos ont leur propre circuit. */
export function validateFeedPost(raw: { kind?: unknown; body?: unknown; delayMinutes?: unknown }): FeedPostInput {
  const kind = String(raw.kind ?? "") as FeedKind;
  if (!FEED_KINDS.includes(kind) || kind === "photos") return { ok: false, error: "Type de publication inconnu." };
  if (kind === "delay") {
    const n = Math.round(Number(raw.delayMinutes));
    if (!Number.isFinite(n) || n < DELAY_MIN_MINUTES || n > DELAY_MAX_MINUTES) {
      return { ok: false, error: `Durée du retard : entre ${DELAY_MIN_MINUTES} et ${DELAY_MAX_MINUTES} minutes.` };
    }
    return { ok: true, kind, body: cleanText(raw.body, MESSAGE_MAX_CHARS) || null, delayMinutes: n };
  }
  if (kind === "message" || kind === "program") {
    const max = kind === "program" ? PROGRAM_MAX_CHARS : MESSAGE_MAX_CHARS;
    const body = cleanText(raw.body, max + 1);
    if (!body) return { ok: false, error: "Le texte est vide." };
    if (body.length > max) return { ok: false, error: `Texte trop long (${max} caractères maximum).` };
    return { ok: true, kind, body, delayMinutes: null };
  }
  return { ok: true, kind, body: cleanText(raw.body, MESSAGE_MAX_CHARS) || null, delayMinutes: null };
}

/** Texte affiché à la famille pour une publication. */
export function feedPostText(p: { kind: string; body: string | null; delayMinutes: number | null }): string {
  switch (p.kind) {
    case "arrives":
      return p.body || "Bien arrivés !";
    case "ok":
      return p.body || "Tout va bien.";
    case "delay":
      return `Petit retard : environ ${p.delayMinutes ?? "?"} min.${p.body ? ` ${p.body}` : ""}`;
    case "departure_back":
      return p.body || "Le groupe a pris la route du retour.";
    case "at_school":
      return p.body || "Le groupe est arrivé à l'établissement.";
    default:
      return p.body || "";
  }
}

/* ─── Photos et droit à l'image ─── */

export type ImageRightStatus = "refuse" | "non_renseigne";
export type ImageRightListed = { ine: string; nom: string; prenom: string; classe?: string; status: ImageRightStatus };

/**
 * Participants dont le droit à l'image n'est PAS « oui » : `false` = refusé, absent = non renseigné (affiché aussi, prudence).
 * Seul `true` explicite dispense.
 */
export function studentsWithoutImageRight(participants: TravelsParticipantEleve[]): ImageRightListed[] {
  const out: ImageRightListed[] = [];
  for (const p of participants) {
    const v = (p as { droitImageOk?: boolean | null }).droitImageOk;
    if (v === true) continue;
    out.push({
      ine: p.ine,
      nom: p.nom,
      prenom: p.prenom,
      ...(p.classe ? { classe: p.classe } : {}),
      status: v === false ? "refuse" : "non_renseigne",
    });
  }
  return out.sort((a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"));
}

/** La case « Aucun de ces élèves n'est reconnaissable » est OBLIGATOIRE dès que la liste n'est pas vide. */
export function checkPhotoPublication(opts: {
  listed: ImageRightListed[];
  acknowledged: boolean;
  photoCount: number;
}): { ok: true } | { ok: false; error: string } {
  if (opts.photoCount < 1) return { ok: false, error: "Ajoutez au moins une photo." };
  if (opts.photoCount > PHOTOS_MAX_PER_POST) return { ok: false, error: `${PHOTOS_MAX_PER_POST} photos au maximum par publication.` };
  if (opts.listed.length > 0 && !opts.acknowledged) {
    return {
      ok: false,
      error: "Cochez la case « Aucun de ces élèves n'est reconnaissable sur ces photos » : certains élèves n'ont pas le droit à l'image.",
    };
  }
  return { ok: true };
}

/* ─── Réactions ─── */

export function isReactionEmoji(v: unknown): v is ReactionEmoji {
  return typeof v === "string" && (REACTION_EMOJIS as readonly string[]).includes(v);
}

/** Réactions d'une famille après bascule : une par type, retirable, jamais deux fois la même. */
export function applyReaction(current: string[], emoji: string, on: boolean): string[] {
  if (!isReactionEmoji(emoji)) return current;
  const has = current.includes(emoji);
  if (on && !has) return [...current, emoji];
  if (!on && has) return current.filter((e) => e !== emoji);
  return current;
}

/** Compteurs sans noms : emoji → nombre de familles, et les réactions de MA famille. */
export function summarizeReactions(rows: { familyKey: string; emoji: string }[], myFamilyKey: string): { counts: Record<string, number>; mine: string[] } {
  const counts: Record<string, number> = {};
  const mine: string[] = [];
  const seen = new Set<string>();
  for (const r of rows) {
    if (!isReactionEmoji(r.emoji)) continue;
    const k = `${r.familyKey}|${r.emoji}`;
    if (seen.has(k)) continue;
    seen.add(k);
    counts[r.emoji] = (counts[r.emoji] || 0) + 1;
    if (r.familyKey === myFamilyKey) mine.push(r.emoji);
  }
  return { counts, mine };
}

/* ─── E-mails regroupés ─── */

export type FeedNotifyState = {
  lastNotifiedAt: Date | null;
  pendingMessages: number;
  pendingPhotos: number;
  notifyCount: number;
  sendFailures?: number;
};

/** Une publication de type photos compte comme « photos », toutes les autres comme « message ». */
export function pendingBucket(kind: string): "photos" | "messages" {
  return kind === "photos" ? "photos" : "messages";
}

export type FeedEmailDecision = { send: false; reason: "nothing" | "too_soon" | "family_cap" | "trip_cap" } | { send: true };

/** Au plus un e-mail par famille et par heure ; plafonds par famille et par voyage. Le reste attend le passage suivant. */
export function planFeedEmail(opts: { state: FeedNotifyState; now: Date; tripEmailsSent: number }): FeedEmailDecision {
  const pending = opts.state.pendingMessages + opts.state.pendingPhotos;
  if (pending <= 0) return { send: false, reason: "nothing" };
  if (opts.tripEmailsSent >= FEED_EMAIL_CAP_PER_TRIP) return { send: false, reason: "trip_cap" };
  if (opts.state.notifyCount >= FEED_EMAIL_CAP_PER_FAMILY) return { send: false, reason: "family_cap" };
  const last = opts.state.lastNotifiedAt;
  if (last && opts.now.getTime() - last.getTime() < FEED_EMAIL_MIN_INTERVAL_MS) return { send: false, reason: "too_soon" };
  return { send: true };
}

export type FeedEmailItem = {
  kind: string;
  body: string | null;
  delayMinutes: number | null;
  authorName: string | null;
  createdAt: Date;
  /** Nombre de photos de la publication (jamais jointes ni affichées dans l'e-mail). */
  photoCount: number;
};

const escHtml = (s: string) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

/** Prénom de l'auteur (premier mot du nom complet) : jamais un nom d'élève, toujours un adulte du voyage. */
export function authorFirstName(name: string | null | undefined): string {
  return String(name ?? "").trim().split(/\s+/)[0]?.slice(0, 40) || "";
}

const SUBJECT_PHRASE: Record<string, string> = {
  arrives: "nous sommes bien arrivés",
  ok: "tout va bien",
  departure_back: "départ du retour",
  at_school: "arrivés à l'établissement",
  program: "programme du jour",
  message: "nouveau message",
};

function itemTime(d: Date): string {
  return d.toLocaleString("fr-FR", { timeZone: "Europe/Paris", day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

/**
 * E-mail EXPLICITE regroupé (au plus un par famille et par heure) : reprend le TEXTE des publications, dans l'ordre
 * chronologique, avec l'heure et le prénom de l'auteur. Les PHOTOS ne sont jamais dans l'e-mail : seulement leur nombre
 * et le lien personnel. Tout texte est échappé ; le texte du prof est repris tel quel (le prof est averti à la saisie).
 * Les publications sans texte ni photo (rien à dire) sont ignorées.
 */
export function buildFeedUpdateEmail(opts: {
  destination: string;
  tripTitle: string;
  school: string;
  link: string;
  items: FeedEmailItem[];
}): { subject: string; html: string; text: string; itemCount: number } | null {
  const items = [...opts.items].sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
  if (items.length === 0) return null;
  const label = opts.destination.trim() ? `Sortie à ${opts.destination.trim()}` : opts.tripTitle.trim() || "Voyage scolaire";
  const photoTotal = items.reduce((n, i) => n + i.photoCount, 0);
  let subjectTail: string;
  if (items.length === 1) {
    const only = items[0];
    if (only.kind === "photos") subjectTail = `${only.photoCount > 1 ? `${only.photoCount} nouvelles photos` : "nouvelle photo"}`;
    else if (only.kind === "delay") subjectTail = `petit retard (~${only.delayMinutes ?? "?"} min)`;
    else subjectTail = SUBJECT_PHRASE[only.kind] || "nouvelle information";
  } else {
    subjectTail = `${items.length} nouvelles informations`;
  }
  const subject = `${label.replace(/[\r\n]+/g, " ").slice(0, 90)} : ${subjectTail}`;

  const textItems = items.filter((i) => i.kind !== "photos");
  const lineOf = (i: FeedEmailItem) => {
    const who = authorFirstName(i.authorName);
    const head = `${itemTime(i.createdAt)}${who ? ` — ${who}` : ""}`;
    return { head, body: feedPostText({ kind: i.kind, body: i.body, delayMinutes: i.delayMinutes }), title: FEED_KIND_LABELS[i.kind as FeedKind] || "" };
  };
  const lines = textItems.map(lineOf);
  const photoLine = photoTotal > 0 ? `${photoTotal} nouvelle${photoTotal > 1 ? "s" : ""} photo${photoTotal > 1 ? "s" : ""} — à voir sur votre page.` : "";

  const htmlItems = lines
    .map(
      (l) =>
        `<li style="margin:0 0 14px 0;"><div style="font-size:12px;color:#64748b;">${escHtml(l.head)}</div><div style="font-size:15px;font-weight:bold;white-space:pre-line;">${escHtml(l.body)}</div></li>`,
    )
    .join("");
  const html = `
    <div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;">
      <p>Bonjour,</p>
      <p>Des nouvelles du voyage <strong>« ${escHtml(opts.tripTitle || label)} »</strong> :</p>
      ${htmlItems ? `<ul style="list-style:none;padding:0;margin:16px 0;">${htmlItems}</ul>` : ""}
      ${photoLine ? `<p style="font-weight:bold;">${escHtml(photoLine)}</p>` : ""}
      <p style="margin: 24px 0;">
        <a href="${escHtml(opts.link)}" style="background:#4338ca;color:#fff;padding:12px 20px;border-radius:10px;text-decoration:none;font-weight:bold;">Ouvrir le fil du voyage</a>
      </p>
      <p style="font-size: 13px; color: #64748b;">Ce lien est personnel et privé : ne le transmettez pas. Le fil est supprimé 7 jours après le retour.</p>
      <p>Cordialement,<br/>${escHtml(opts.school)}</p>
    </div>`;
  const text = [
    "Bonjour,",
    "",
    `Des nouvelles du voyage « ${opts.tripTitle || label} » :`,
    "",
    ...lines.flatMap((l) => [`${l.head}`, l.body, ""]),
    ...(photoLine ? [photoLine, ""] : []),
    `Ouvrir le fil : ${opts.link}`,
    "",
    "Ce lien est personnel et privé : ne le transmettez pas. Le fil est supprimé 7 jours après le retour.",
    "",
    opts.school,
  ].join("\n");
  return { subject, html, text, itemCount: items.length };
}

/** Quand une publication arrive : compteurs à ajouter aux familles (résumé « N nouveautés »). */
export function pendingDelta(kind: string): { messages: number; photos: number } {
  return pendingBucket(kind) === "photos" ? { messages: 0, photos: 1 } : { messages: 1, photos: 0 };
}

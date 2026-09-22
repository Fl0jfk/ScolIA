"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";
import { parisDateKey, parseParisDateTime } from "@/app/lib/paris-time";
import { RDV_BOOK_CONFIRM_PHRASE } from "@/app/lib/rdv-inscription-types";

export type PublicRdvSlot = {
  eventId: string;
  startAt: string;
  endAt: string;
  title: string;
};

export type PublicRdvLevel = { id: string; label: string };

export type PublicRdvPageProps = {
  directionSlug: string;
  title: string;
  intro: string;
  consentLabel: string;
  location: string;
  directionLabel: string;
  directriceDisplayName: string | null;
  levels: PublicRdvLevel[];
  initialSlots: PublicRdvSlot[];
  initialError: string | null;
};

type MatchParent = {
  prenom: string;
  nom: string;
  email: string | null;
  rang: number;
};

type MatchCandidate = {
  id: string;
  prenom: string;
  nom: string;
  classe: string | null;
  status: string;
  parents: MatchParent[];
};

type MatchChoice =
  | { kind: "eleve"; id: string; label: string; parents: MatchParent[] }
  | { kind: "create" }
  | null;

type OrigineEtab = {
  codeRne: string;
  label: string;
  adresse: string | null;
};

function namesFromParents(
  parents: MatchParent[],
  attendee: "madame" | "monsieur" | "les_deux",
  parentEmail: string,
): { first: string; last: string } {
  const email = parentEmail.trim().toLowerCase();
  const byEmail = parents.find((p) => p.email && p.email === email);
  const sorted = [...parents].sort((a, b) => a.rang - b.rang);
  if (attendee === "les_deux" && sorted.length >= 2) {
    return {
      first: `${sorted[0]!.prenom} & ${sorted[1]!.prenom}`,
      last:
        sorted[0]!.nom.toLowerCase() === sorted[1]!.nom.toLowerCase()
          ? sorted[0]!.nom
          : `${sorted[0]!.nom} / ${sorted[1]!.nom}`,
    };
  }
  if (attendee === "monsieur") {
    const p = sorted[1] || byEmail || sorted[0];
    return p ? { first: p.prenom, last: p.nom } : { first: "", last: "" };
  }
  // madame (ou défaut)
  const p = byEmail || sorted[0];
  return p ? { first: p.prenom, last: p.nom } : { first: "", last: "" };
}

function formatHm(iso: string): string {
  return new Date(iso).toLocaleTimeString("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDayChip(dayKey: string): { weekday: string; dayNum: string; month: string } {
  const d = parseParisDateTime(dayKey, "12:00");
  if (!d) return { weekday: "", dayNum: dayKey, month: "" };
  return {
    weekday: d.toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", weekday: "short" }),
    dayNum: d.toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", day: "numeric" }),
    month: d.toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", month: "short" }),
  };
}

function formatDayLong(dayKey: string): string {
  const d = parseParisDateTime(dayKey, "12:00");
  if (!d) return dayKey;
  return d.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
  });
}

function formatSlotRange(startAt: string, endAt: string): string {
  return `${new Date(startAt).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  })} – ${formatHm(endAt)}`;
}

const fieldClass =
  "mt-1.5 w-full rounded-lg border border-slate-200 bg-white px-3.5 py-2.5 text-slate-900 shadow-sm outline-none transition placeholder:text-slate-400 focus:border-sky-500 focus:ring-2 focus:ring-sky-100 disabled:bg-slate-50 disabled:text-slate-400";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function RdvInscriptionPublicClient({
  directionSlug,
  title,
  intro,
  consentLabel,
  location,
  directionLabel,
  directriceDisplayName,
  levels,
  initialSlots,
  initialError,
}: PublicRdvPageProps) {
  const [slots, setSlots] = useState(initialSlots);
  const [loadError, setLoadError] = useState(initialError);
  const [eventId, setEventId] = useState("");
  const [selectedDay, setSelectedDay] = useState<string>("");
  const [studentFirstName, setStudentFirstName] = useState("");
  const [studentLastName, setStudentLastName] = useState("");
  const [parentEmail, setParentEmail] = useState("");
  const [parentPhone, setParentPhone] = useState("");
  const [parentFirstName, setParentFirstName] = useState("");
  const [parentLastName, setParentLastName] = useState("");
  const [rdvAttendee, setRdvAttendee] = useState<"madame" | "monsieur" | "les_deux" | "">("");
  const [niveauId, setNiveauId] = useState(levels[0]?.id || "");
  const [emailChildren, setEmailChildren] = useState<MatchCandidate[]>([]);
  const [candidates, setCandidates] = useState<MatchCandidate[] | null>(null);
  const [emailLinked, setEmailLinked] = useState(false);
  const [showIdentitySearch, setShowIdentitySearch] = useState(false);
  const [matchChoice, setMatchChoice] = useState<MatchChoice>(null);
  const [homeEtablissement, setHomeEtablissement] = useState<OrigineEtab | null>(null);
  const [emailVerified, setEmailVerified] = useState(false);
  const [emailGateLoading, setEmailGateLoading] = useState(true);
  const [emailPending, setEmailPending] = useState(false);
  const [emailGateBusy, setEmailGateBusy] = useState(false);
  const [matchBusy, setMatchBusy] = useState(false);
  const [studentDateNaissance, setStudentDateNaissance] = useState("");
  const [hasPap, setHasPap] = useState<"yes" | "no" | "">("");
  const [papBringToRdv, setPapBringToRdv] = useState(false);
  const [papFile, setPapFile] = useState<{
    s3Key: string;
    fileName: string;
    mimeType: string;
  } | null>(null);
  const [papUploadBusy, setPapUploadBusy] = useState(false);
  const [origineCp, setOrigineCp] = useState("");
  const [origineDept, setOrigineDept] = useState("");
  const [origineQuery, setOrigineQuery] = useState("");
  const [origineResults, setOrigineResults] = useState<OrigineEtab[]>([]);
  const [origineSelected, setOrigineSelected] = useState<OrigineEtab | null>(null);
  const [origineBusy, setOrigineBusy] = useState(false);
  const [consent, setConsent] = useState(false);
  const [confirmTyped, setConfirmTyped] = useState("");
  const [honeypot, setHoneypot] = useState("");
  const [busy, setBusy] = useState(false);
  const [existingBookings, setExistingBookings] = useState<
    Array<{
      id: string;
      status: string;
      startAt: string;
      endAt: string;
      niveauLabel: string | null;
    }>
  >([]);
  const [existingBusy, setExistingBusy] = useState(false);
  const [modifyExisting, setModifyExisting] = useState(false);
  const [done, setDone] = useState<{
    pending?: boolean;
    startAt: string;
    endAt: string;
    mailWarning?: string;
  } | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

  const emailOk = EMAIL_RE.test(parentEmail.trim());
  const confirmPhraseOk =
    confirmTyped
      .trim()
      .toUpperCase()
      .normalize("NFD")
      .replace(/\p{M}/gu, "") === RDV_BOOK_CONFIRM_PHRASE;

  const byDay = useMemo(() => {
    const map = new Map<string, PublicRdvSlot[]>();
    for (const s of slots) {
      const key = parisDateKey(s.startAt);
      const list = map.get(key) || [];
      list.push(s);
      map.set(key, list);
    }
    for (const list of map.values()) {
      list.sort((a, b) => a.startAt.localeCompare(b.startAt));
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [slots]);

  const dayKeys = useMemo(() => byDay.map(([k]) => k), [byDay]);

  useEffect(() => {
    if (!dayKeys.length) {
      setSelectedDay("");
      setEventId("");
      return;
    }
    setSelectedDay((prev) => (prev && dayKeys.includes(prev) ? prev : dayKeys[0]!));
  }, [dayKeys]);

  const daySlots = useMemo(() => {
    if (!selectedDay) return [];
    return byDay.find(([k]) => k === selectedDay)?.[1] || [];
  }, [byDay, selectedDay]);

  useEffect(() => {
    if (!daySlots.length) {
      setEventId("");
      return;
    }
    setEventId((prev) => (prev && daySlots.some((s) => s.eventId === prev) ? prev : ""));
  }, [daySlots]);

  useEffect(() => {
    if (!levels.length) return;
    if (!levels.some((l) => l.id === niveauId)) {
      setNiveauId(levels[0]!.id);
    }
  }, [levels, niveauId]);

  const selectedSlot = useMemo(
    () => slots.find((s) => s.eventId === eventId) || null,
    [slots, eventId],
  );

  const matchReady = matchChoice?.kind === "eleve";
  const hasExisting = existingBookings.length > 0;

  const refreshSlots = useCallback(async () => {
    try {
      const res = await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/slots`);
      const data = (await res.json()) as { slots?: PublicRdvSlot[]; error?: string };
      if (!res.ok) {
        setLoadError(data.error || "Impossible de charger les créneaux.");
        setSlots([]);
        return;
      }
      setLoadError(null);
      setSlots(data.slots || []);
    } catch {
      setLoadError("Impossible de charger les créneaux.");
    }
  }, [directionSlug]);

  useEffect(() => {
    if (!initialError && initialSlots.length === 0) {
      void refreshSlots();
    }
  }, [initialError, initialSlots.length, refreshSlots]);

  function applySessionCandidates(
    listRaw: MatchCandidate[] | undefined,
    home: OrigineEtab | null | undefined,
  ) {
    const list = (listRaw || []).map((c) => ({
      ...c,
      parents: Array.isArray(c.parents) ? c.parents : [],
    }));
    setEmailChildren(list);
    setCandidates(null);
    setHomeEtablissement(home || null);
    setMatchChoice(null);
    setOrigineSelected(null);
    setOrigineResults([]);
    setRdvAttendee("");
    setParentFirstName("");
    setParentLastName("");
    setStudentDateNaissance("");
    setEmailLinked(list.length > 0);
    setShowIdentitySearch(list.length === 0);
  }

  const loadEmailSession = useCallback(async () => {
    setEmailGateLoading(true);
    try {
      const res = await fetch(
        `/api/rdv-inscription/${encodeURIComponent(directionSlug)}/email-session`,
      );
      const data = (await res.json()) as {
        verified?: boolean;
        email?: string;
        candidates?: MatchCandidate[];
        homeEtablissement?: OrigineEtab | null;
      };
      if (res.ok && data.verified && data.email) {
        setEmailVerified(true);
        setEmailPending(false);
        setParentEmail(data.email);
        applySessionCandidates(data.candidates, data.homeEtablissement);
      } else {
        setEmailVerified(false);
      }
    } catch {
      setEmailVerified(false);
    } finally {
      setEmailGateLoading(false);
    }
  }, [directionSlug]);

  useEffect(() => {
    void loadEmailSession();
  }, [loadEmailSession]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    const emailError = params.get("email_error");
    if (emailError) {
      setFormError(
        emailError === "rate"
          ? "Trop de tentatives. Réessayez dans quelques minutes."
          : emailError === "error"
            ? "Lien de confirmation invalide. Demandez un nouvel e-mail."
            : decodeURIComponent(emailError),
      );
      params.delete("email_error");
      const next = `${window.location.pathname}${params.toString() ? `?${params}` : ""}`;
      window.history.replaceState({}, "", next);
    }
    if (params.get("email_ok") === "1") {
      params.delete("email_ok");
      const next = `${window.location.pathname}${params.toString() ? `?${params}` : ""}`;
      window.history.replaceState({}, "", next);
      void loadEmailSession();
    }
  }, [loadEmailSession]);

  async function onStartEmailGate(e: React.FormEvent) {
    e.preventDefault();
    setFormError(null);
    if (!emailOk) {
      setFormError("Saisissez un e-mail parent valide.");
      return;
    }
    setEmailGateBusy(true);
    try {
      const res = await fetch(
        `/api/rdv-inscription/${encodeURIComponent(directionSlug)}/email-start`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ parentEmail }),
        },
      );
      const data = (await res.json()) as {
        pending?: boolean;
        message?: string;
        mailWarning?: string;
        error?: string;
      };
      if (!res.ok) {
        setFormError(data.error || "Envoi impossible.");
        return;
      }
      setEmailPending(true);
      if (data.mailWarning) {
        setFormError(data.mailWarning);
      }
    } catch {
      setFormError("Erreur réseau — réessayez.");
    } finally {
      setEmailGateBusy(false);
    }
  }

  async function onChangeEmail() {
    setFormError(null);
    try {
      await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/email-session`, {
        method: "DELETE",
      });
    } catch {
      /* ignore */
    }
    setEmailVerified(false);
    setEmailPending(false);
    setCandidates(null);
    setEmailChildren([]);
    setEmailLinked(false);
    setShowIdentitySearch(false);
    setMatchChoice(null);
    setHomeEtablissement(null);
    setOrigineSelected(null);
    setStudentFirstName("");
    setStudentLastName("");
    setStudentDateNaissance("");
    setParentPhone("");
    setParentFirstName("");
    setParentLastName("");
    setRdvAttendee("");
  }

  const matchedParents =
    matchChoice?.kind === "eleve" ? matchChoice.parents : [];
  const showAttendeeChoice = Boolean(
    matchChoice?.kind === "eleve" && matchedParents.length > 0,
  );
  const showParentNameFields = Boolean(matchChoice?.kind === "eleve" && !showAttendeeChoice);

  function applyAttendee(next: "madame" | "monsieur" | "les_deux") {
    setRdvAttendee(next);
    if (matchChoice?.kind !== "eleve") return;
    const names = namesFromParents(matchChoice.parents, next, parentEmail);
    if (names.first) setParentFirstName(names.first);
    if (names.last) setParentLastName(names.last);
  }

  function selectMatchedEleve(c: MatchCandidate) {
    setMatchChoice({
      kind: "eleve",
      id: c.id,
      label: `${c.prenom} ${c.nom}${c.classe ? ` (${c.classe})` : ""}`,
      parents: c.parents || [],
    });
    setStudentFirstName(c.prenom);
    setStudentLastName(c.nom);
    if (homeEtablissement) {
      setOrigineSelected(homeEtablissement);
      setOrigineResults([]);
    }
    setRdvAttendee("");
    setParentFirstName("");
    setParentLastName("");
    setExistingBookings([]);
    setModifyExisting(false);
    void loadExistingBooking(c.id);
  }

  async function loadExistingBooking(eleveId: string) {
    setExistingBusy(true);
    try {
      const res = await fetch(
        `/api/rdv-inscription/${encodeURIComponent(directionSlug)}/existing?eleveId=${encodeURIComponent(eleveId)}&parentEmail=${encodeURIComponent(parentEmail.trim())}`,
      );
      const data = (await res.json()) as {
        bookings?: Array<{
          id: string;
          status: string;
          startAt: string;
          endAt: string;
          niveauLabel: string | null;
        }>;
        booking?: {
          id: string;
          status: string;
          startAt: string;
          endAt: string;
          niveauLabel: string | null;
        } | null;
        error?: string;
      };
      if (!res.ok) {
        setExistingBookings([]);
        return;
      }
      const list =
        Array.isArray(data.bookings) && data.bookings.length
          ? data.bookings
          : data.booking
            ? [data.booking]
            : [];
      setExistingBookings(list);
      setModifyExisting(false);
    } catch {
      setExistingBookings([]);
    } finally {
      setExistingBusy(false);
    }
  }

  async function onSearchByIdentity() {
    setFormError(null);
    if (!studentFirstName.trim() && !studentLastName.trim()) {
      setFormError("Indiquez au moins le prénom ou le nom de l’élève.");
      return;
    }
    if (!studentDateNaissance.trim()) {
      setFormError("Indiquez la date de naissance de l’élève.");
      return;
    }
    setMatchBusy(true);
    setMatchChoice(null);
    try {
      const res = await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/match`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          parentEmail,
          parentPhone,
          studentFirstName,
          studentLastName,
          dateNaissance: studentDateNaissance,
        }),
      });
      const data = (await res.json()) as {
        candidates?: MatchCandidate[];
        homeEtablissement?: OrigineEtab | null;
        error?: string;
      };
      if (!res.ok) {
        setFormError(data.error || "Recherche impossible.");
        return;
      }
      const list = (data.candidates || []).map((c) => ({
        ...c,
        parents: Array.isArray(c.parents) ? c.parents : [],
      }));
      setCandidates(list);
      if (data.homeEtablissement) setHomeEtablissement(data.homeEtablissement);
      if (list.length === 0) {
        setFormError(
          "Aucun élève trouvé. Vérifiez la date de naissance et l’orthographe exacte de la préinscription École Directe (majuscules / accents acceptés), puis réessayez.",
        );
      }
    } catch {
      setFormError("Erreur réseau — réessayez.");
    } finally {
      setMatchBusy(false);
    }
  }

  async function searchOrigine() {
    setFormError(null);
    setOrigineBusy(true);
    setOrigineSelected(null);
    try {
      const params = new URLSearchParams();
      if (origineCp.trim()) params.set("cp", origineCp.trim());
      if (origineDept.trim()) params.set("dept", origineDept.trim());
      if (origineQuery.trim()) params.set("q", origineQuery.trim());
      params.set("limit", "100");
      if (!params.has("cp") && !params.has("dept") && !params.has("q")) {
        setFormError("Indiquez un code postal, un département ou un nom d’établissement.");
        return;
      }
      const res = await fetch(`/api/fiches-dialogue/public/etablissements?${params}`);
      const data = (await res.json()) as {
        etablissements?: OrigineEtab[];
        error?: string;
        hint?: string;
      };
      if (!res.ok) {
        setFormError(data.error || "Recherche établissement impossible.");
        return;
      }
      setOrigineResults(data.etablissements || []);
      if (!(data.etablissements || []).length) {
        setFormError(data.hint || "Aucun établissement trouvé — affinez le code postal ou le nom.");
      }
    } catch {
      setFormError("Erreur réseau — réessayez.");
    } finally {
      setOrigineBusy(false);
    }
  }

  async function uploadPapFile(file: File) {
    setFormError(null);
    setPapUploadBusy(true);
    try {
      const prep = await fetch("/api/rdv-inscription/pap-upload", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          fileName: file.name,
          contentType: file.type || "application/pdf",
          size: file.size,
        }),
      });
      const prepJ = (await prep.json()) as {
        uploadUrl?: string;
        s3Key?: string;
        fileName?: string;
        contentType?: string;
        error?: string;
      };
      if (!prep.ok || !prepJ.uploadUrl || !prepJ.s3Key) {
        throw new Error(prepJ.error || "Préparation upload impossible");
      }
      const put = await fetch(prepJ.uploadUrl, {
        method: "PUT",
        headers: { "Content-Type": file.type || "application/pdf" },
        body: file,
      });
      if (!put.ok) throw new Error("Échec envoi du fichier");
      setPapFile({
        s3Key: prepJ.s3Key,
        fileName: prepJ.fileName || file.name,
        mimeType: prepJ.contentType || file.type || "application/pdf",
      });
      setPapBringToRdv(false);
    } catch (e) {
      setFormError(e instanceof Error ? e.message : "Upload PAP impossible");
    } finally {
      setPapUploadBusy(false);
    }
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setFormError(null);
    if (!matchChoice || matchChoice.kind !== "eleve") {
      setFormError("Confirmez l’identité de l’élève avant de réserver.");
      return;
    }
    if (!studentFirstName.trim() || !studentLastName.trim()) {
      setFormError("Indiquez le prénom et le nom de l’élève.");
      return;
    }
    if (showIdentitySearch && !studentDateNaissance.trim()) {
      setFormError("Indiquez la date de naissance utilisée pour la recherche.");
      return;
    }
    if (!parentPhone.trim()) {
      setFormError("Indiquez le téléphone du parent.");
      return;
    }
    if (!origineSelected) {
      setFormError("Sélectionnez l’établissement d’origine.");
      return;
    }
    if (hasPap !== "yes" && hasPap !== "no") {
      setFormError("Indiquez si votre enfant a un PAP.");
      return;
    }
    if (hasPap === "yes" && !papFile && !papBringToRdv) {
      setFormError(
        "Déposez le PAP maintenant, ou confirmez que vous l’apporterez le jour J.",
      );
      return;
    }
    if (!parentFirstName.trim() || !parentLastName.trim()) {
      setFormError(
        showAttendeeChoice
          ? "Indiquez qui sera présent au rendez-vous (les noms se remplissent alors automatiquement)."
          : "Indiquez le prénom et le nom du parent.",
      );
      return;
    }
    if (showAttendeeChoice && !rdvAttendee) {
      setFormError("Indiquez qui sera présent au rendez-vous.");
      return;
    }
    if (!niveauId) {
      setFormError("Choisissez le niveau demandé.");
      return;
    }
    if (!eventId) {
      setFormError("Choisissez un créneau.");
      return;
    }
    if (!consent) {
      setFormError("Merci d’accepter le traitement de vos coordonnées.");
      return;
    }
    if (!confirmPhraseOk) {
      setFormError(`Pour confirmer, saisissez ${RDV_BOOK_CONFIRM_PHRASE} dans le champ prévu.`);
      return;
    }
    setBusy(true);
    try {
      const res = await fetch(`/api/rdv-inscription/${encodeURIComponent(directionSlug)}/book`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eventId,
          studentFirstName,
          studentLastName,
          parentEmail,
          parentPhone,
          parentFirstName,
          parentLastName,
          rdvAttendee: showAttendeeChoice ? rdvAttendee || null : null,
          niveauId,
          eleveId: matchChoice.id,
          createNew: false,
          studentDateNaissance: studentDateNaissance.trim() || null,
          hasPap,
          papS3Key: papFile?.s3Key || null,
          papFileName: papFile?.fileName || null,
          papMimeType: papFile?.mimeType || null,
          papBringToRdv: hasPap === "yes" && !papFile,
          etablissementOrigineRne: origineSelected.codeRne,
          etablissementOrigineLabel: origineSelected.label,
          etablissementOrigineAdresse: origineSelected.adresse,
          confirmTyped,
          consent: true,
          website: honeypot,
        }),
      });
      const data = (await res.json()) as {
        success?: boolean;
        pending?: boolean;
        error?: string;
        startAt?: string;
        endAt?: string;
        mailWarning?: string;
      };
      if (!res.ok || !data.success) {
        setFormError(data.error || "Réservation impossible.");
        if (res.status === 409 || res.status === 410) {
          await refreshSlots();
        }
        return;
      }
      setDone({
        pending: false,
        startAt: data.startAt || "",
        endAt: data.endAt || "",
        mailWarning: data.mailWarning,
      });
    } catch {
      setFormError("Erreur réseau — réessayez.");
    } finally {
      setBusy(false);
    }
  }

  if (done) {
    const isPending = done.pending === true;
    return (
      <div className="min-h-screen bg-[#f7f8fa]">
        <RentreePublicHeader />
        <main className="mx-auto max-w-lg px-4 py-14 text-center">
          <div
            className={`mx-auto flex h-14 w-14 items-center justify-center rounded-full text-2xl ${
              isPending ? "bg-sky-100 text-sky-700" : "bg-emerald-100 text-emerald-700"
            }`}
          >
            {isPending ? "✉" : "✓"}
          </div>
          <h1 className="mt-5 text-2xl font-bold tracking-tight text-slate-900">
            {isPending ? "Vérifiez votre e-mail" : "Rendez-vous confirmé"}
          </h1>
          <p className="mt-2 text-slate-600">
            {directionLabel}
            {directriceDisplayName ? ` · ${directriceDisplayName}` : ""}
          </p>
          {done.startAt ? (
            <p className="mt-4 rounded-xl bg-white px-4 py-3 text-sm font-medium text-slate-800 shadow-sm ring-1 ring-slate-200/80">
              {formatSlotRange(done.startAt, done.endAt || done.startAt)}
            </p>
          ) : null}
          <p className="mt-5 text-sm leading-relaxed text-slate-500">
            {isPending
              ? "Un e-mail vient de vous être envoyé. Cliquez sur le lien pour valider votre créneau (valable 2 heures). Sans validation, le créneau sera libéré."
              : "Un e-mail de confirmation avec fichier calendrier (.ics) vous a été envoyé."}
          </p>
          {done.mailWarning ? (
            <p className="mt-3 text-sm text-amber-800">{done.mailWarning}</p>
          ) : null}
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#f7f8fa]">
      <RentreePublicHeader />
      <main className="mx-auto max-w-xl px-4 py-8 sm:py-10">
        <header className="mb-8">
          <p className="text-xs font-semibold uppercase tracking-[0.14em] text-sky-700">
            {directionLabel}
            {directriceDisplayName ? ` · ${directriceDisplayName}` : ""}
          </p>
          <h1 className="mt-2 text-3xl font-bold tracking-tight text-slate-900 sm:text-[2rem]">
            {title}
          </h1>
          {intro ? (
            <p className="mt-3 text-[15px] leading-relaxed text-slate-600 whitespace-pre-line">
              {intro}
            </p>
          ) : null}
          {location ? (
            <p className="mt-3 inline-flex items-center gap-1.5 rounded-full bg-white px-3 py-1 text-sm text-slate-600 ring-1 ring-slate-200/80">
              <span className="text-slate-400">Lieu</span>
              <span className="font-medium text-slate-800">{location}</span>
            </p>
          ) : null}
          <p
            role="note"
            className="mt-5 rounded-xl border-2 border-red-600 bg-red-50 px-4 py-3.5 text-center text-base font-extrabold leading-snug text-red-700 sm:text-lg"
          >
            La présence de l’enfant au rendez-vous est indispensable.
          </p>
        </header>

        {loadError ? (
          <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
            {loadError}
          </div>
        ) : null}

        {!loadError && slots.length === 0 ? (
          <div className="rounded-xl bg-white px-5 py-8 text-center text-slate-600 shadow-sm ring-1 ring-slate-200/80">
            Aucun créneau disponible pour le moment.
            <br />
            <span className="text-sm text-slate-500">
              Revenez plus tard ou contactez l’établissement.
            </span>
          </div>
        ) : null}

        {slots.length > 0 && emailGateLoading ? (
          <div className="rounded-xl bg-white px-5 py-8 text-center text-sm text-slate-500 shadow-sm ring-1 ring-slate-200/80">
            Vérification de votre session…
          </div>
        ) : null}

        {slots.length > 0 && !emailGateLoading && !emailVerified ? (
          <form onSubmit={onStartEmailGate} className="space-y-6">
            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                1 · Confirmez votre e-mail
              </h2>
              <p className="mt-2 text-sm text-slate-500">
                Avant d’accéder au formulaire, nous vous envoyons un lien de confirmation.
              </p>
              {emailPending ? (
                <div className="mt-4 space-y-3 rounded-xl bg-sky-50 px-4 py-3 text-sm text-sky-950 ring-1 ring-sky-100">
                  <p className="font-semibold">E-mail envoyé à {parentEmail.trim().toLowerCase()}</p>
                  <p>
                    Ouvrez votre boîte et cliquez sur « Confirmer mon e-mail ». Vous pourrez ensuite
                    choisir l’enfant et réserver un créneau.
                  </p>
                  <button
                    type="submit"
                    disabled={emailGateBusy || !emailOk}
                    className="rounded-lg bg-sky-800 px-3 py-2 text-xs font-bold text-white hover:bg-sky-900 disabled:opacity-50"
                  >
                    {emailGateBusy ? "Envoi…" : "Renvoyer le lien"}
                  </button>
                </div>
              ) : (
                <div className="mt-4 grid gap-4">
                  <label className="block text-sm">
                    <span className="font-semibold text-slate-800">E-mail du parent</span>
                    <input
                      required
                      type="email"
                      className={fieldClass}
                      value={parentEmail}
                      onChange={(e) => setParentEmail(e.target.value)}
                      autoComplete="email"
                      placeholder="prenom.nom@email.fr"
                    />
                  </label>
                  <button
                    type="submit"
                    disabled={emailGateBusy || !emailOk}
                    className="w-full rounded-xl bg-sky-700 px-4 py-3 text-sm font-bold text-white transition hover:bg-sky-800 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {emailGateBusy ? "Envoi…" : "Recevoir le lien de confirmation"}
                  </button>
                </div>
              )}
            </section>
            {formError ? (
              <p className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
                {formError}
              </p>
            ) : null}
          </form>
        ) : null}

        {slots.length > 0 && emailVerified ? (
          <form onSubmit={onSubmit} className="space-y-6">
            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                1 · E-mail confirmé
              </h2>
              <p className="mt-3 text-sm text-slate-700">
                <span className="font-semibold text-emerald-800">{parentEmail}</span>
                <button
                  type="button"
                  onClick={() => void onChangeEmail()}
                  className="ml-3 text-xs font-semibold text-sky-700 underline-offset-2 hover:underline"
                >
                  Changer d’e-mail
                </button>
              </p>
              <label className="mt-4 block text-sm">
                <span className="font-semibold text-slate-800">Téléphone du parent</span>
                <input
                  required
                  type="tel"
                  className={fieldClass}
                  value={parentPhone}
                  onChange={(e) => setParentPhone(e.target.value)}
                  autoComplete="tel"
                  placeholder="06 …"
                />
              </label>
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                2 · Pour quel enfant ?
              </h2>

              {emailLinked && !showIdentitySearch ? (
                <>
                  <p className="mt-2 text-sm text-slate-500">
                    Enfants liés à cet e-mail. Confirmez celui pour qui vous prenez rendez-vous.
                  </p>
                  {emailChildren.length > 0 ? (
                    <div className="mt-4 space-y-2">
                      {emailChildren.map((c) => {
                        const selected =
                          matchChoice?.kind === "eleve" && matchChoice.id === c.id;
                        return (
                          <button
                            key={c.id}
                            type="button"
                            onClick={() => selectMatchedEleve(c)}
                            className={`w-full rounded-xl px-4 py-3 text-left text-sm transition ${
                              selected
                                ? "bg-sky-700 text-white shadow-sm"
                                : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white"
                            }`}
                          >
                            <span className="font-semibold">
                              Est-ce bien {c.prenom} {c.nom.toUpperCase()} ?
                            </span>
                            {c.classe ? (
                              <span
                                className={`mt-0.5 block text-xs ${
                                  selected ? "text-sky-100" : "text-slate-500"
                                }`}
                              >
                                {c.classe}
                              </span>
                            ) : null}
                          </button>
                        );
                      })}
                    </div>
                  ) : null}
                </>
              ) : null}

              {showIdentitySearch ? (
                <div className="mt-2 space-y-4">
                  <p className="text-sm text-slate-500">
                    Cet e-mail n’est pas encore lié à un foyer connu. Indiquez l’identité de l’élève{" "}
                    <strong>exactement comme sur la préinscription École Directe</strong> (casse et
                    accents acceptés). Il faut la date de naissance et au moins le nom ou le prénom.
                  </p>
                  <div className="grid gap-4 sm:grid-cols-2">
                    <label className="block text-sm">
                      <span className="font-semibold text-slate-800">Prénom de l’élève</span>
                      <input
                        className={fieldClass}
                        value={studentFirstName}
                        onChange={(e) => {
                          setStudentFirstName(e.target.value);
                          setMatchChoice(null);
                        }}
                        autoComplete="given-name"
                        placeholder="Prénom"
                      />
                    </label>
                    <label className="block text-sm">
                      <span className="font-semibold text-slate-800">Nom de l’élève</span>
                      <input
                        className={fieldClass}
                        value={studentLastName}
                        onChange={(e) => {
                          setStudentLastName(e.target.value);
                          setMatchChoice(null);
                        }}
                        autoComplete="family-name"
                        placeholder="Nom"
                      />
                    </label>
                  </div>
                  <label className="block text-sm">
                    <span className="font-semibold text-slate-800">Date de naissance</span>
                    <input
                      required
                      type="date"
                      className={fieldClass}
                      value={studentDateNaissance}
                      onChange={(e) => {
                        setStudentDateNaissance(e.target.value);
                        setMatchChoice(null);
                      }}
                    />
                  </label>
                  <button
                    type="button"
                    disabled={
                      matchBusy ||
                      (!studentFirstName.trim() && !studentLastName.trim()) ||
                      !studentDateNaissance.trim()
                    }
                    onClick={() => void onSearchByIdentity()}
                    className="w-full rounded-xl bg-slate-800 px-4 py-3 text-sm font-bold text-white transition hover:bg-slate-900 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {matchBusy ? "Recherche…" : "Rechercher mon enfant"}
                  </button>

                  {candidates && candidates.length > 0 ? (
                    <div className="space-y-2">
                      <p className="text-sm text-slate-600">Confirmez le bon enfant :</p>
                      {candidates.map((c) => {
                        const selected =
                          matchChoice?.kind === "eleve" && matchChoice.id === c.id;
                        return (
                          <button
                            key={c.id}
                            type="button"
                            onClick={() => selectMatchedEleve(c)}
                            className={`w-full rounded-xl px-4 py-3 text-left text-sm transition ${
                              selected
                                ? "bg-sky-700 text-white shadow-sm"
                                : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white"
                            }`}
                          >
                            <span className="font-semibold">
                              Est-ce bien {c.prenom} {c.nom.toUpperCase()} ?
                            </span>
                            {c.classe ? (
                              <span
                                className={`mt-0.5 block text-xs ${
                                  selected ? "text-sky-100" : "text-slate-500"
                                }`}
                              >
                                {c.classe}
                              </span>
                            ) : null}
                          </button>
                        );
                      })}
                    </div>
                  ) : null}
                </div>
              ) : null}

              {matchChoice?.kind === "eleve" ? (
                <p className="mt-3 text-sm text-emerald-700">
                  Élève confirmé : <strong>{matchChoice.label}</strong>
                </p>
              ) : null}
            </section>

            {matchReady ? (
              <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
                <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                  2b · Parent au rendez-vous
                </h2>
                {showAttendeeChoice ? (
                  <>
                    <p className="mt-2 text-sm text-slate-500">
                      Les parents sont déjà connus pour cet enfant. Indiquez seulement qui sera
                      présent.
                    </p>
                    <fieldset className="mt-4 block text-sm">
                      <legend className="font-semibold text-slate-800">
                        Qui sera présent au rendez-vous ?
                      </legend>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {(
                          [
                            { id: "madame", label: "Madame" },
                            { id: "monsieur", label: "Monsieur" },
                            { id: "les_deux", label: "Les deux" },
                          ] as const
                        ).map((opt) => (
                          <label
                            key={opt.id}
                            className={`inline-flex cursor-pointer items-center gap-2 rounded-xl border px-3 py-2 text-sm font-medium transition ${
                              rdvAttendee === opt.id
                                ? "border-sky-600 bg-sky-50 text-sky-900"
                                : "border-slate-200 bg-white text-slate-700 hover:border-slate-300"
                            }`}
                          >
                            <input
                              type="radio"
                              name="rdvAttendee"
                              className="sr-only"
                              checked={rdvAttendee === opt.id}
                              onChange={() => applyAttendee(opt.id)}
                            />
                            {opt.label}
                          </label>
                        ))}
                      </div>
                    </fieldset>
                    {parentFirstName.trim() && parentLastName.trim() ? (
                      <p className="mt-3 text-sm text-emerald-700">
                        Présent :{" "}
                        <strong>
                          {parentFirstName} {parentLastName}
                        </strong>
                      </p>
                    ) : null}
                  </>
                ) : showParentNameFields ? (
                  <>
                    <p className="mt-2 text-sm text-slate-500">
                      Indiquez le prénom et le nom du parent qui prend rendez-vous (pas besoin de
                      préciser Madame / Monsieur / les deux).
                    </p>
                    <div className="mt-4 grid gap-4 sm:grid-cols-2">
                      <label className="block text-sm">
                        <span className="font-semibold text-slate-800">Prénom du parent</span>
                        <input
                          required
                          className={fieldClass}
                          value={parentFirstName}
                          onChange={(e) => setParentFirstName(e.target.value)}
                          autoComplete="given-name"
                          placeholder="Prénom"
                        />
                      </label>
                      <label className="block text-sm">
                        <span className="font-semibold text-slate-800">Nom du parent</span>
                        <input
                          required
                          className={fieldClass}
                          value={parentLastName}
                          onChange={(e) => setParentLastName(e.target.value)}
                          autoComplete="family-name"
                          placeholder="Nom (peut différer de l’élève)"
                        />
                      </label>
                    </div>
                  </>
                ) : null}
              </section>
            ) : null}

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                3 · Établissement d’origine
              </h2>
              {matchChoice?.kind === "eleve" &&
              homeEtablissement &&
              origineSelected?.codeRne === homeEtablissement.codeRne ? (
                <div className="mt-3 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
                  Élève déjà scolarisé chez nous : établissement d’origine renseigné
                  automatiquement — <strong>{origineSelected.label}</strong>
                </div>
              ) : (
                <>
              <p className="mt-2 text-sm text-slate-500">
                Filtrez par code postal (recommandé) ou département, puis choisissez l’établissement.
              </p>
              <div className="mt-4 grid gap-3 sm:grid-cols-3">
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">Code postal</span>
                  <input
                    className={fieldClass}
                    value={origineCp}
                    onChange={(e) => setOrigineCp(e.target.value)}
                    placeholder="76500"
                    inputMode="numeric"
                    disabled={!matchReady}
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">Département (UAI)</span>
                  <input
                    className={fieldClass}
                    value={origineDept}
                    onChange={(e) => setOrigineDept(e.target.value)}
                    placeholder="076"
                    disabled={!matchReady}
                  />
                </label>
                <label className="block text-sm">
                  <span className="font-semibold text-slate-800">Nom (optionnel)</span>
                  <input
                    className={fieldClass}
                    value={origineQuery}
                    onChange={(e) => setOrigineQuery(e.target.value)}
                    placeholder="Collège…"
                    disabled={!matchReady}
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={!matchReady || origineBusy}
                onClick={() => void searchOrigine()}
                className="mt-3 rounded-xl bg-slate-800 px-4 py-2.5 text-sm font-bold text-white hover:bg-slate-900 disabled:opacity-50"
              >
                {origineBusy ? "Recherche…" : "Rechercher l’établissement"}
              </button>
              {origineResults.length > 0 ? (
                <ul className="mt-3 max-h-72 space-y-1 overflow-y-auto">
                  {origineResults.map((e) => {
                    const selected = origineSelected?.codeRne === e.codeRne;
                    return (
                      <li key={e.codeRne}>
                        <button
                          type="button"
                          onClick={() => setOrigineSelected(e)}
                          className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                            selected
                              ? "bg-sky-700 text-white"
                              : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white"
                          }`}
                        >
                          <span className="font-semibold">{e.label}</span>
                          {e.adresse ? (
                            <span
                              className={`mt-0.5 block text-xs ${
                                selected ? "text-sky-100" : "text-slate-500"
                              }`}
                            >
                              {e.adresse}
                            </span>
                          ) : null}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              ) : null}
              {origineSelected ? (
                <p className="mt-3 text-sm text-emerald-700">
                  Sélection : <strong>{origineSelected.label}</strong>
                </p>
              ) : null}
                </>
              )}
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                4 · PAP (Plan d’accompagnement personnalisé)
              </h2>
              <p className="mt-2 text-sm text-slate-600">
                Votre enfant a-t-il un PAP ?
              </p>
              <div className="mt-3 flex flex-wrap gap-3">
                <button
                  type="button"
                  disabled={!matchReady}
                  onClick={() => {
                    setHasPap("no");
                    setPapFile(null);
                    setPapBringToRdv(false);
                  }}
                  className={`rounded-xl px-4 py-2.5 text-sm font-bold ${
                    hasPap === "no"
                      ? "bg-slate-800 text-white"
                      : "bg-slate-50 text-slate-700 ring-1 ring-slate-200"
                  }`}
                >
                  Non
                </button>
                <button
                  type="button"
                  disabled={!matchReady}
                  onClick={() => setHasPap("yes")}
                  className={`rounded-xl px-4 py-2.5 text-sm font-bold ${
                    hasPap === "yes"
                      ? "bg-sky-700 text-white"
                      : "bg-slate-50 text-slate-700 ring-1 ring-slate-200"
                  }`}
                >
                  Oui
                </button>
              </div>
              {hasPap === "yes" ? (
                <div className="mt-4 space-y-3">
                  <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
                    Le PAP doit être <strong>présent au rendez-vous</strong> : la direction le traite
                    en même temps que l’inscription. Si vous ne l’avez pas sous la main maintenant,
                    il faudra penser à <strong>l’apporter le jour J</strong>.
                  </p>
                  <label
                    className={`flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed px-4 py-8 text-center transition ${
                      papFile
                        ? "border-emerald-300 bg-emerald-50"
                        : "border-slate-300 bg-slate-50 hover:border-sky-400"
                    }`}
                    onDragOver={(e) => e.preventDefault()}
                    onDrop={(e) => {
                      e.preventDefault();
                      const f = e.dataTransfer.files?.[0];
                      if (f) void uploadPapFile(f);
                    }}
                  >
                    <span className="text-sm font-semibold text-slate-800">
                      {papUploadBusy
                        ? "Envoi…"
                        : papFile
                          ? `Déposé : ${papFile.fileName}`
                          : "Glissez-déposez le PAP (PDF / image) — facultatif maintenant"}
                    </span>
                    <span className="mt-1 text-xs text-slate-500">
                      ou cliquez pour choisir un fichier — sinon engagement ci-dessous
                    </span>
                    <input
                      type="file"
                      className="hidden"
                      accept="application/pdf,image/jpeg,image/png,image/webp"
                      disabled={papUploadBusy || !matchReady}
                      onChange={(e) => {
                        const f = e.target.files?.[0];
                        if (f) void uploadPapFile(f);
                        e.target.value = "";
                      }}
                    />
                  </label>
                  {!papFile ? (
                    <label className="flex items-start gap-3 text-sm text-slate-700">
                      <input
                        type="checkbox"
                        className="mt-1 h-4 w-4 rounded border-slate-300 text-sky-700"
                        checked={papBringToRdv}
                        onChange={(e) => setPapBringToRdv(e.target.checked)}
                      />
                      <span>
                        Je n’ai pas le fichier sous la main : je m’engage à{" "}
                        <strong>apporter le PAP le jour du rendez-vous</strong>.
                      </span>
                    </label>
                  ) : (
                    <button
                      type="button"
                      className="text-sm font-semibold text-slate-600 underline"
                      onClick={() => setPapFile(null)}
                    >
                      Retirer le fichier
                    </button>
                  )}
                </div>
              ) : null}
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                5 · Niveau demandé
              </h2>
              <label className="mt-4 block text-sm">
                <span className="font-semibold text-slate-800">Classe / formation</span>
                <select
                  required
                  className={fieldClass}
                  value={niveauId}
                  onChange={(e) => setNiveauId(e.target.value)}
                  disabled={!matchReady}
                >
                  {levels.map((l) => (
                    <option key={l.id} value={l.id}>
                      {l.label}
                    </option>
                  ))}
                </select>
              </label>
            </section>

            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <div className="flex flex-wrap items-end justify-between gap-2">
                <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
                  6 · Choisir un créneau
                </h2>
                <p className="text-xs text-slate-500">
                  {slots.length} créneau{slots.length > 1 ? "x" : ""} · {dayKeys.length} jour
                  {dayKeys.length > 1 ? "s" : ""}
                </p>
              </div>

              <p
                role="note"
                className="mt-4 rounded-xl border-2 border-red-600 bg-red-50 px-4 py-3 text-center text-base font-extrabold leading-snug text-red-700 sm:text-lg"
              >
                La présence de l’enfant au rendez-vous est indispensable.
              </p>

              {matchReady && existingBusy ? (
                <p className="mt-4 text-sm text-slate-500">Vérification d’un rendez-vous existant…</p>
              ) : null}

              {matchReady && hasExisting && !modifyExisting ? (
                <div className="mt-4 rounded-xl border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950">
                  <p className="font-bold">
                    {existingBookings.length > 1
                      ? `Vous avez déjà ${existingBookings.length} rendez-vous`
                      : "Vous avez déjà un rendez-vous"}
                  </p>
                  <ul className="mt-2 list-disc space-y-1 pl-5">
                    {existingBookings.map((b) => (
                      <li key={b.id}>
                        {formatSlotRange(b.startAt, b.endAt)}
                        {b.niveauLabel ? ` · ${b.niveauLabel}` : ""}
                        {b.status === "pending" ? " (en attente de validation)" : ""}
                      </li>
                    ))}
                  </ul>
                  <p className="mt-2 text-xs text-amber-900/80">
                    Si vous choisissez un autre créneau, les anciens seront libérés
                    automatiquement.
                  </p>
                  <button
                    type="button"
                    onClick={() => {
                      setModifyExisting(true);
                      setEventId("");
                    }}
                    className="mt-3 rounded-lg bg-amber-800 px-3 py-2 text-xs font-bold text-white hover:bg-amber-900"
                  >
                    Modifier mon créneau
                  </button>
                </div>
              ) : null}

              {matchReady && hasExisting && modifyExisting ? (
                <div className="mt-4 rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-950">
                  <p className="font-semibold">Modification du rendez-vous</p>
                  <p className="mt-1 text-xs text-sky-900/80">
                    Créneau{existingBookings.length > 1 ? "x" : ""} actuel
                    {existingBookings.length > 1 ? "s" : ""} :{" "}
                    {existingBookings
                      .map((b) => formatSlotRange(b.startAt, b.endAt))
                      .join(" · ")}
                    . En confirmant un nouveau créneau, le ou les anciens seront remis
                    disponibles.
                  </p>
                </div>
              ) : null}

              {!matchReady ? (
                <p className="mt-4 text-sm text-slate-500">
                  Confirmez d’abord l’élève pour débloquer les créneaux.
                </p>
              ) : hasExisting && !modifyExisting ? (
                <p className="mt-4 text-sm text-slate-500">
                  Votre créneau est déjà réservé. Cliquez sur « Modifier mon créneau » pour en
                  choisir un autre.
                </p>
              ) : (
                <>
                  <p className="mt-4 text-sm font-semibold text-slate-800">Jour</p>
                  <div
                    className="-mx-1 mt-2 flex gap-2 overflow-x-auto px-1 pb-1 [scrollbar-width:thin]"
                    role="listbox"
                    aria-label="Choisir un jour"
                  >
                    {byDay.map(([day, dayList]) => {
                      const chip = formatDayChip(day);
                      const active = day === selectedDay;
                      return (
                        <button
                          key={day}
                          type="button"
                          role="option"
                          aria-selected={active}
                          onClick={() => setSelectedDay(day)}
                          className={`flex min-w-[4.5rem] shrink-0 flex-col items-center rounded-xl px-3 py-2.5 text-center transition ${
                            active
                              ? "bg-sky-700 text-white shadow-sm"
                              : "bg-slate-50 text-slate-700 ring-1 ring-slate-200 hover:bg-slate-100"
                          }`}
                        >
                          <span
                            className={`text-[11px] font-semibold uppercase tracking-wide ${
                              active ? "text-sky-100" : "text-slate-500"
                            }`}
                          >
                            {chip.weekday}
                          </span>
                          <span className="mt-0.5 text-xl font-bold leading-none">
                            {chip.dayNum}
                          </span>
                          <span
                            className={`mt-1 text-[11px] font-medium capitalize ${
                              active ? "text-sky-100" : "text-slate-500"
                            }`}
                          >
                            {chip.month}
                          </span>
                          <span
                            className={`mt-1.5 text-[10px] ${
                              active ? "text-sky-200" : "text-slate-400"
                            }`}
                          >
                            {dayList.length} crén.
                          </span>
                        </button>
                      );
                    })}
                  </div>

                  {selectedDay ? (
                    <>
                      <p className="mt-5 text-sm font-semibold text-slate-800">
                        Horaires — {formatDayLong(selectedDay)}
                      </p>
                      <div className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3">
                        {daySlots.map((s) => {
                          const selected = s.eventId === eventId;
                          return (
                            <button
                              key={s.eventId}
                              type="button"
                              onClick={() => setEventId(s.eventId)}
                              className={`rounded-xl px-3 py-3 text-center text-sm font-semibold transition ${
                                selected
                                  ? "bg-sky-700 text-white shadow-sm ring-2 ring-sky-700 ring-offset-2"
                                  : "bg-slate-50 text-slate-800 ring-1 ring-slate-200 hover:bg-white hover:ring-sky-300"
                              }`}
                            >
                              <span className="block text-base tabular-nums">
                                {formatHm(s.startAt)}
                              </span>
                              <span
                                className={`mt-0.5 block text-xs font-medium tabular-nums ${
                                  selected ? "text-sky-100" : "text-slate-500"
                                }`}
                              >
                                → {formatHm(s.endAt)}
                              </span>
                            </button>
                          );
                        })}
                      </div>
                    </>
                  ) : null}

                  {selectedSlot ? (
                    <p className="mt-4 rounded-lg bg-sky-50 px-3 py-2 text-sm text-sky-950">
                      Sélection :{" "}
                      <strong>
                        {formatSlotRange(selectedSlot.startAt, selectedSlot.endAt)}
                      </strong>
                    </p>
                  ) : (
                    <p className="mt-4 text-sm text-slate-500">
                      Sélectionnez un jour puis un horaire.
                    </p>
                  )}
                </>
              )}
            </section>

            {hasExisting && !modifyExisting ? null : (
            <section className="rounded-2xl bg-white p-5 shadow-sm ring-1 ring-slate-200/80 sm:p-6">
              <label className="flex items-start gap-3 text-sm leading-relaxed text-slate-700">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4 rounded border-slate-300 text-sky-700 focus:ring-sky-500"
                  checked={consent}
                  onChange={(e) => setConsent(e.target.checked)}
                />
                <span>{consentLabel}</span>
              </label>

              <label className="mt-5 block text-sm text-slate-700">
                <span className="font-medium text-slate-900">
                  Confirmation — saisissez{" "}
                  <span className="font-mono tracking-wide text-sky-800">{RDV_BOOK_CONFIRM_PHRASE}</span>
                </span>
                <span className="mt-1 block text-xs text-slate-500">
                  Pour éviter une réservation trop rapide, retapez ce mot exact.
                  {hasExisting
                    ? " L’ancien créneau sera libéré au profit du nouveau."
                    : ""}
                </span>
                <input
                  type="text"
                  value={confirmTyped}
                  onChange={(e) => setConfirmTyped(e.target.value)}
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={RDV_BOOK_CONFIRM_PHRASE}
                  className="mt-2 w-full rounded-xl border border-slate-300 bg-white px-3 py-2.5 text-sm text-slate-900 shadow-sm placeholder:text-slate-400 focus:border-sky-500 focus:outline-none focus:ring-2 focus:ring-sky-500/30"
                />
              </label>

              <input
                type="text"
                name="website"
                value={honeypot}
                onChange={(e) => setHoneypot(e.target.value)}
                className="absolute -left-[9999px] h-0 w-0 opacity-0"
                tabIndex={-1}
                autoComplete="off"
                aria-hidden
              />

              {formError ? (
                <p className="mt-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
                  {formError}
                </p>
              ) : null}

              <button
                type="submit"
                disabled={
                  busy ||
                  !matchReady ||
                  !eventId ||
                  !origineSelected ||
                  !hasPap ||
                  !parentFirstName.trim() ||
                  !parentLastName.trim() ||
                  (showAttendeeChoice && !rdvAttendee) ||
                  !consent ||
                  !confirmPhraseOk
                }
                className="mt-5 w-full rounded-xl bg-sky-700 px-4 py-3.5 text-sm font-bold text-white shadow-sm transition hover:bg-sky-800 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {busy
                  ? "Réservation…"
                  : hasExisting
                    ? "Confirmer le nouveau créneau"
                    : "Confirmer le rendez-vous"}
              </button>
            </section>
            )}
          </form>
        ) : null}
      </main>
    </div>
  );
}

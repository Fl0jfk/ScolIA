"use client";

import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type { StageConvention } from "@/app/lib/stage-types";
import type { StageClassPeriod, StagePeriodReminder } from "@/app/lib/stage-periods-config";
import type { StageConventionCard } from "@/app/lib/stage-signature-summary";
import { STAGE_CONVENTION_STATUS_LABELS, formatCompanyAddress } from "@/app/lib/stage-types";
import { formatPeriodRangeFr } from "@/app/lib/stage-schedule";
import {
  clearPreconventionDeviceMemory,
  readPreconventionDeviceMemory,
  writePreconventionDeviceMemory,
} from "@/app/lib/stage-preconvention-device-memory";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";
import StageOtpCodeInput from "@/app/components/stages/StageOtpCodeInput";
import StageSchedulePanel from "@/app/components/stages/StageSchedulePanel";
import {
  usePublicSiteIdentity,
  type PublicSiteIdentity,
} from "@/app/contexts/public-site-identity";

type PeriodAvailability = StageClassPeriod & {
  used: boolean;
  conventionId?: string;
};

type StudentDossier = {
  schoolYear: string;
  conventions: StageConventionCard[];
  availablePeriods: PeriodAvailability[];
  canCreateNew: boolean;
};

type StudentPreview = {
  firstName: string;
  lastName: string;
  className: string;
  photoUrl?: string | null;
  parent1Email?: string | null;
  parent2Email?: string | null;
  parentPhone?: string | null;
  parent2Phone?: string | null;
  studentEmail?: string | null;
};

function formatIsoDateFr(iso: string): string {
  const raw = iso.trim().slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(raw)) return iso;
  return new Date(`${raw}T12:00:00`).toLocaleDateString("fr-FR");
}

function studentInitials(firstName: string, lastName: string): string {
  const a = firstName.trim().charAt(0);
  const b = lastName.trim().charAt(0);
  return `${a}${b}`.toUpperCase() || "?";
}


function extractTokenFromStudentLink(studentLink: string): string | null {
  try {
    const url = studentLink.startsWith("http")
      ? new URL(studentLink)
      : new URL(studentLink, "http://local");
    return url.searchParams.get("token");
  } catch {
    return null;
  }
}

function StagePreconventionPublicContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const tokenFromUrl = searchParams.get("token") || "";
  const contextIdentity = usePublicSiteIdentity();
  const [fetchedIdentity, setFetchedIdentity] = useState<PublicSiteIdentity | null>(null);

  const [step, setStep] = useState<"identity" | "dashboard" | "form">(
    tokenFromUrl ? "form" : "identity",
  );
  const [nom, setNom] = useState("");
  const [prenom, setPrenom] = useState("");
  const [dateNaissance, setDateNaissance] = useState("");
  const [classe, setClasse] = useState("");
  const [classOptions, setClassOptions] = useState<string[]>([]);
  const [studentPreview, setStudentPreview] = useState<StudentPreview | null>(null);
  const [photoFailed, setPhotoFailed] = useState(false);
  const [studentPhotoUrl, setStudentPhotoUrl] = useState<string | null>(null);
  const [dossier, setDossier] = useState<StudentDossier | null>(null);
  const [token, setToken] = useState(tokenFromUrl);
  const [convention, setConvention] = useState<StageConvention | null>(null);
  const [readOnly, setReadOnly] = useState(false);
  const [canEditTutorEmail, setCanEditTutorEmail] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [reminders, setReminders] = useState<StagePeriodReminder[]>([]);
  const [officialPeriods, setOfficialPeriods] = useState<StageClassPeriod[]>([]);
  const [rejectNote, setRejectNote] = useState<string | null>(null);
  const [selectedPeriodId, setSelectedPeriodId] = useState("");
  const [signatureSummary, setSignatureSummary] = useState<
    import("@/app/lib/stage-signature-summary").StageSignatureSummary | null
  >(null);

  const [parent1Email, setParent1Email] = useState("");
  const [parent2Email, setParent2Email] = useState("");
  const [editingParentEmail, setEditingParentEmail] = useState(false);
  const [parentEmailVerified, setParentEmailVerified] = useState(false);
  const [showParentCode, setShowParentCode] = useState(false);
  const [parentCode, setParentCode] = useState("");
  const [infoMsg, setInfoMsg] = useState<string | null>(null);
  const [tutorEmailEdit, setTutorEmailEdit] = useState("");
  const [restoringDevice, setRestoringDevice] = useState(() => {
    if (tokenFromUrl) return false;
    return Boolean(readPreconventionDeviceMemory());
  });
  const deviceRestoreStarted = useRef(false);

  useEffect(() => {
    if (contextIdentity) return;
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/site/public", { cache: "no-store" });
        const data = (await res.json()) as PublicSiteIdentity;
        if (!cancelled && res.ok) setFetchedIdentity(data);
      } catch {
        /* logo / nom établissement optionnels */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [contextIdentity]);

  const siteIdentity = contextIdentity ?? fetchedIdentity;
  const schoolName = siteIdentity?.name?.trim() || siteIdentity?.shortName?.trim() || "";
  const logoUrl = siteIdentity?.headerLogoUrl?.trim() || "";

  function rememberIdentity(fields?: {
    nom?: string;
    prenom?: string;
    dateNaissance?: string;
    classe?: string;
  }) {
    writePreconventionDeviceMemory({
      nom: (fields?.nom ?? nom).trim(),
      prenom: (fields?.prenom ?? prenom).trim(),
      dateNaissance: (fields?.dateNaissance ?? dateNaissance).trim(),
      classe: (fields?.classe ?? classe).trim() || undefined,
    });
  }

  function forgetIdentity() {
    clearPreconventionDeviceMemory();
    setStudentPreview(null);
    setPhotoFailed(false);
    setStudentPhotoUrl(null);
    setDossier(null);
    setNom("");
    setPrenom("");
    setDateNaissance("");
    setClasse("");
    setClassOptions([]);
    setParent1Email("");
    setParent2Email("");
    setStep("identity");
    setError(null);
    setInfoMsg(null);
  }

  function applyStageContext(ctx: unknown) {
    if (!ctx || typeof ctx !== "object") {
      setReminders([]);
      setOfficialPeriods([]);
      return;
    }
    const o = ctx as { reminders?: StagePeriodReminder[]; periods?: StageClassPeriod[] };
    setReminders(Array.isArray(o.reminders) ? o.reminders : []);
    setOfficialPeriods(Array.isArray(o.periods) ? o.periods : []);
  }

  const loadConvention = useCallback(async (activeToken: string) => {
    if (!activeToken) return;
    setError(null);
    const res = await fetch(`/api/stages/public/student?token=${encodeURIComponent(activeToken)}`, {
      cache: "no-store",
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Lien invalide");
    setConvention(data.convention);
    setReadOnly(data.readOnly === true);
    setCanEditTutorEmail(data.canEditTutorEmail === true);
    setSignatureSummary(data.signatureSummary ?? null);
    setParentEmailVerified(data.parentEmailVerified === true);
    setTutorEmailEdit(String(data.convention?.company?.tutorEmail ?? ""));
    applyStageContext(data.stageContext);
    const nextPhoto =
      typeof data.studentPhotoUrl === "string" && data.studentPhotoUrl.trim()
        ? data.studentPhotoUrl.trim()
        : null;
    setStudentPhotoUrl(nextPhoto);
    setPhotoFailed(false);
    if (data.convention?.status === "admin_rejected" && data.convention.adminReview?.note) {
      setRejectNote(data.convention.adminReview.note);
    } else {
      setRejectNote(null);
    }
    const memory = readPreconventionDeviceMemory();
    const student = data.convention?.student as
      | { firstName?: string; lastName?: string; className?: string }
      | undefined;
    if (memory && student) {
      const samePerson =
        memory.prenom.trim().toLowerCase() === String(student.firstName ?? "").trim().toLowerCase() &&
        memory.nom.trim().toLowerCase() === String(student.lastName ?? "").trim().toLowerCase();
      if (samePerson) {
        setNom(memory.nom);
        setPrenom(memory.prenom);
        setDateNaissance(memory.dateNaissance);
        setClasse(memory.classe || String(student.className ?? "") || "");
      }
    }
    setStep("form");
  }, []);

  useEffect(() => {
    if (tokenFromUrl) {
      setToken(tokenFromUrl);
      void loadConvention(tokenFromUrl).catch((e: unknown) =>
        setError(e instanceof Error ? e.message : "Erreur"),
      );
      return;
    }

    if (deviceRestoreStarted.current) return;
    const memory = readPreconventionDeviceMemory();
    if (!memory) {
      setRestoringDevice(false);
      return;
    }
    deviceRestoreStarted.current = true;
    setRestoringDevice(true);
    setNom(memory.nom);
    setPrenom(memory.prenom);
    setDateNaissance(memory.dateNaissance);
    if (memory.classe) setClasse(memory.classe);
    void identifyWithCredentials(memory)
      .catch((e: unknown) => {
        clearPreconventionDeviceMemory();
        setStep("identity");
        setError(
          e instanceof Error
            ? e.message
            : "Impossible de vous reconnaître automatiquement. Identifiez-vous à nouveau.",
        );
      })
      .finally(() => setRestoringDevice(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- restauration unique au montage
  }, [tokenFromUrl, loadConvention]);

  function identityPayload(extra?: Record<string, unknown>) {
    const memory = readPreconventionDeviceMemory();
    return {
      nom: nom.trim() || memory?.nom || "",
      prenom: prenom.trim() || memory?.prenom || "",
      dateNaissance: dateNaissance || memory?.dateNaissance || "",
      classe: (classe.trim() || memory?.classe || undefined) as string | undefined,
      ...extra,
    };
  }

  function applyIdentifySuccess(
    data: {
      studentPreview: StudentPreview;
      dossier: StudentDossier;
      stageContext?: unknown;
    },
    identity?: {
      nom: string;
      prenom: string;
      dateNaissance: string;
      classe?: string;
    },
  ) {
    const preview = data.studentPreview;
    setStudentPreview(preview);
    setPhotoFailed(false);
    setStudentPhotoUrl(preview.photoUrl?.trim() || null);
    setParent1Email(String(preview.parent1Email ?? ""));
    setParent2Email(String(preview.parent2Email ?? ""));
    setEditingParentEmail(false);
    setDossier(data.dossier);
    applyStageContext(data.stageContext);
    setClassOptions([]);
    const nextClasse = (identity?.classe || preview.className || classe).trim();
    if (nextClasse) setClasse(nextClasse);
    rememberIdentity({
      nom: (identity?.nom || preview.lastName || nom).trim(),
      prenom: (identity?.prenom || preview.firstName || prenom).trim(),
      dateNaissance: (identity?.dateNaissance || dateNaissance).trim(),
      classe: nextClasse || undefined,
    });
    setStep("dashboard");
  }

  async function identifyWithCredentials(creds: {
    nom: string;
    prenom: string;
    dateNaissance: string;
    classe?: string;
  }) {
    const res = await fetch("/api/stages/public/preconvention", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        nom: creds.nom.trim(),
        prenom: creds.prenom.trim(),
        dateNaissance: creds.dateNaissance,
        classe: creds.classe?.trim() || undefined,
        action: "identify",
      }),
    });
    const data = await res.json();
    if (data?.needsClass === true) {
      const options = Array.isArray(data.candidates)
        ? data.candidates
            .map((c: { className?: string }) => String(c?.className ?? "").trim())
            .filter(Boolean)
        : [];
      setClassOptions(options);
      setClasse("");
      setNom(creds.nom);
      setPrenom(creds.prenom);
      setDateNaissance(creds.dateNaissance);
      setStep("identity");
      setError(
        String(data.message ?? "Plusieurs élèves correspondent. Sélectionnez votre classe."),
      );
      return { ok: false as const, needsClass: true as const };
    }
    if (!res.ok) throw new Error(data?.error || "Erreur");
    setNom(creds.nom.trim());
    setPrenom(creds.prenom.trim());
    setDateNaissance(creds.dateNaissance);
    if (creds.classe) setClasse(creds.classe.trim());
    applyIdentifySuccess(data, creds);
    return { ok: true as const };
  }

  async function verifyIdentity(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await identifyWithCredentials({ nom, prenom, dateNaissance, classe });
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function createNewStage() {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(
          identityPayload({
            action: "create",
            periodId: selectedPeriodId || undefined,
            parent1Email: parent1Email.trim() || undefined,
            parent2Email: parent2Email.trim() || undefined,
          }),
        ),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");

      const newToken = extractTokenFromStudentLink(String(data.studentLink ?? ""));
      if (!newToken) throw new Error("Impossible d'ouvrir le formulaire.");

      setToken(newToken);
      setDone(false);
      router.replace(`/stages/preconvention?token=${encodeURIComponent(newToken)}`);
      await loadConvention(newToken);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function openExistingStage(card: StageConventionCard) {
    if (!card.studentAccessToken) {
      setError("Ce dossier n'est plus accessible en ligne.");
      return;
    }
    setToken(card.studentAccessToken);
    setDone(false);
    router.replace(`/stages/preconvention?token=${encodeURIComponent(card.studentAccessToken)}`);
    void loadConvention(card.studentAccessToken);
  }

  async function save(action: "save" | "submit") {
    if (!convention || !token) return;
    setBusy(true);
    setError(null);
    setInfoMsg(null);
    try {
      if (action === "submit" && !parentEmailVerified) {
        const sendRes = await fetch("/api/stages/public/student", {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ token, action: "send_parent_code", convention }),
        });
        const sendData = await sendRes.json();
        if (!sendRes.ok) throw new Error(sendData?.error || "Erreur envoi code");
        setConvention(sendData.convention);
        setShowParentCode(true);
        setInfoMsg(
          "Un code à 6 chiffres a été envoyé à l'adresse du responsable légal. Saisissez-le ci-dessous pour confirmer.",
        );
        return;
      }

      const res = await fetch("/api/stages/public/student", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, action, convention }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setConvention(data.convention);
      if (action === "submit") setDone(true);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function confirmParentCode() {
    if (!convention || !token) return;
    setBusy(true);
    setError(null);
    setInfoMsg(null);
    try {
      const res = await fetch("/api/stages/public/student", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token,
          action: "confirm_parent_code",
          code: parentCode,
          convention,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Code invalide");
      const verifiedConvention = data.convention as StageConvention;
      setConvention(verifiedConvention);
      setParentEmailVerified(true);
      setShowParentCode(false);

      const submitRes = await fetch("/api/stages/public/student", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token,
          action: "submit",
          convention: verifiedConvention,
        }),
      });
      const submitData = await submitRes.json();
      if (!submitRes.ok) throw new Error(submitData?.error || "Erreur envoi administratif");
      setConvention(submitData.convention);
      setDone(true);
      setInfoMsg(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function saveTutorEmail() {
    if (!token) return;
    setBusy(true);
    setError(null);
    setInfoMsg(null);
    try {
      const res = await fetch("/api/stages/public/student", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token,
          action: "update_tutor_email",
          tutorEmail: tutorEmailEdit,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setConvention(data.convention);
      setInfoMsg("E-mail du tuteur mis à jour — demande de signature renvoyée.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function backToDashboard() {
    setToken("");
    setConvention(null);
    setDone(false);
    setRejectNote(null);
    setShowParentCode(false);
    router.replace("/stages/preconvention");
    const memory = readPreconventionDeviceMemory();
    const creds = {
      nom: nom.trim() || memory?.nom || "",
      prenom: prenom.trim() || memory?.prenom || "",
      dateNaissance: dateNaissance || memory?.dateNaissance || "",
      classe: classe.trim() || memory?.classe || undefined,
    };
    if (!creds.nom || !creds.prenom || !creds.dateNaissance) {
      setStep("identity");
      return;
    }
    setBusy(true);
    void identifyWithCredentials(creds)
      .catch((e: unknown) => {
        setStep("identity");
        setError(e instanceof Error ? e.message : "Erreur");
      })
      .finally(() => setBusy(false));
  }

  if (restoringDevice) {
    return (
      <main className="min-h-screen flex items-center justify-center p-6">
        <p className="text-sm text-stone-600">Reconnaissance de votre appareil…</p>
      </main>
    );
  }

  if (step === "form" && !convention && !error) {
    return <main className="min-h-screen flex items-center justify-center p-6">Chargement…</main>;
  }

  return (
    <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
      <div className="mx-auto max-w-2xl rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <header className="flex flex-col items-center text-center sm:flex-row sm:items-center sm:text-left gap-4">
          {logoUrl ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={logoUrl}
              alt={schoolName || "Logo de l'établissement"}
              className="h-16 w-auto max-w-[140px] object-contain shrink-0"
            />
          ) : null}
          <div className="min-w-0">
            {schoolName ? (
              <p className="text-xs font-bold uppercase tracking-wider text-[#2F6B4A]">
                {schoolName}
              </p>
            ) : null}
            <h1 className="text-2xl font-black text-[#1F3D2B]">Préconvention de stage</h1>
          </div>
        </header>

        {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}
        {infoMsg && (
          <p className="mt-4 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
            {infoMsg}
          </p>
        )}

        {step === "identity" && !token && (
          <form onSubmit={(e) => void verifyIdentity(e)} className="mt-6 space-y-4 text-sm">
            <p className="text-xs text-stone-600">
              Nom, prénom et date de naissance comme sur le bulletin.
            </p>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">Nom *</span>
              <input
                className="mt-1 w-full rounded-lg border px-3 py-2 uppercase"
                placeholder="ex. DUPONT"
                value={nom}
                onChange={(e) => {
                  setNom(e.target.value);
                  setClassOptions([]);
                }}
                autoComplete="family-name"
                required
              />
            </label>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">Prénom *</span>
              <input
                className="mt-1 w-full rounded-lg border px-3 py-2"
                placeholder="ex. Léa"
                value={prenom}
                onChange={(e) => {
                  setPrenom(e.target.value);
                  setClassOptions([]);
                }}
                autoComplete="given-name"
                required
              />
            </label>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">Date de naissance *</span>
              <input
                type="date"
                className="mt-1 w-full rounded-lg border px-3 py-2"
                value={dateNaissance}
                onChange={(e) => {
                  setDateNaissance(e.target.value);
                  setClassOptions([]);
                }}
                required
              />
            </label>
            {classOptions.length > 0 && (
              <label className="block">
                <span className="text-xs font-semibold text-stone-600">Classe *</span>
                <select
                  className="mt-1 w-full rounded-lg border px-3 py-2"
                  value={classe}
                  onChange={(e) => setClasse(e.target.value)}
                  required
                >
                  <option value="">Choisir votre classe…</option>
                  {classOptions.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <button
              type="submit"
              disabled={busy || (classOptions.length > 0 && !classe.trim())}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Vérification…" : "Accéder à mes stages →"}
            </button>
          </form>
        )}

        {step === "dashboard" && studentPreview && dossier && (
          <div className="mt-6 space-y-6 text-sm">
            <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="flex min-w-0 items-start gap-3">
                  <div className="h-14 w-14 shrink-0 overflow-hidden rounded-full border-2 border-white bg-[#2F6B4A]/15 shadow-sm">
                    {(studentPhotoUrl || studentPreview.photoUrl) && !photoFailed ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img
                        src={studentPhotoUrl || studentPreview.photoUrl || ""}
                        alt={`${studentPreview.firstName} ${studentPreview.lastName}`}
                        className="h-full w-full object-cover"
                        onError={() => setPhotoFailed(true)}
                      />
                    ) : (
                      <div
                        className="flex h-full w-full items-center justify-center text-sm font-black text-[#1F3D2B]"
                        aria-hidden
                      >
                        {studentInitials(studentPreview.firstName, studentPreview.lastName)}
                      </div>
                    )}
                  </div>
                  <div className="min-w-0">
                    <p className="text-xs font-bold uppercase tracking-wide text-emerald-800">
                      Élève reconnu
                    </p>
                    <p className="mt-1 text-base font-black text-[#1F3D2B]">
                      {studentPreview.firstName} {studentPreview.lastName}
                    </p>
                    <p className="text-stone-600">
                      {studentPreview.className} · Année {dossier.schoolYear}
                    </p>
                    {(studentPreview.parentPhone || studentPreview.parent2Phone) && (
                      <p className="mt-1 text-xs text-stone-600">
                        Tél. responsable :{" "}
                        {[studentPreview.parentPhone, studentPreview.parent2Phone]
                          .filter(Boolean)
                          .join(" · ")}
                      </p>
                    )}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={forgetIdentity}
                  className="shrink-0 text-xs font-semibold text-stone-500 underline"
                >
                  Ce n&apos;est pas moi
                </button>
              </div>
              <p className="mt-2 text-[11px] text-emerald-800/80">
                Cet appareil se souvient de votre identification pour éviter de ressaisir vos infos.
              </p>
            </div>

            <div className="rounded-xl border-2 border-rose-300 bg-rose-50 px-4 py-4 space-y-3">
              <p className="text-sm font-black text-rose-950">
                Important — convention à signer
              </p>
              <p className="text-xs text-rose-900 leading-relaxed">
                La convention de stage sera envoyée à l&apos;adresse e-mail du responsable légal
                ci-dessous. Vérifiez qu&apos;elle est correcte, par exemple celle du parent qui
                pourra signer.
              </p>
              {!editingParentEmail ? (
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="text-sm text-rose-950">
                    <p>
                      <span className="font-semibold">Responsable 1 :</span>{" "}
                      {parent1Email || (
                        <span className="italic text-rose-700">non renseigné</span>
                      )}
                    </p>
                    {parent2Email && (
                      <p className="mt-1">
                        <span className="font-semibold">Responsable 2 :</span> {parent2Email}
                      </p>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={() => setEditingParentEmail(true)}
                    className="shrink-0 rounded-lg border border-rose-400 bg-white px-3 py-1.5 text-xs font-bold text-rose-900"
                  >
                    Modifier
                  </button>
                </div>
              ) : (
                <div className="space-y-2">
                  <label className="block text-xs font-semibold text-rose-950">
                    E-mail responsable légal 1 *
                    <input
                      type="email"
                      className="mt-1 w-full rounded-lg border border-rose-300 px-3 py-2 text-sm"
                      value={parent1Email}
                      onChange={(e) => setParent1Email(e.target.value)}
                      required
                    />
                  </label>
                  <label className="block text-xs font-semibold text-rose-950">
                    E-mail responsable légal 2 (optionnel)
                    <input
                      type="email"
                      className="mt-1 w-full rounded-lg border border-rose-300 px-3 py-2 text-sm"
                      value={parent2Email}
                      onChange={(e) => setParent2Email(e.target.value)}
                    />
                  </label>
                  <button
                    type="button"
                    onClick={() => setEditingParentEmail(false)}
                    className="rounded-lg bg-rose-800 px-3 py-1.5 text-xs font-bold text-white"
                  >
                    Enregistrer ces adresses
                  </button>
                </div>
              )}
            </div>

            {(reminders.length > 0 || officialPeriods.length > 0) ? (
              <section className="rounded-xl border border-amber-200 bg-amber-50/80 p-4 space-y-2">
                <h2 className="text-sm font-bold text-amber-900">
                  Rappels — dates habituelles pour votre classe
                </h2>
                <p className="text-xs text-amber-900/90 leading-relaxed">
                  Ces dates sont indicatives (périodes prévues pour votre classe). Vous pouvez
                  aussi demander un stage à d&apos;autres dates : l&apos;établissement acceptera ou
                  refusera ensuite.
                </p>
                {officialPeriods.map((p) => (
                  <p key={p.id} className="text-xs text-amber-900">
                    <strong>{p.label}</strong> : du{" "}
                    {new Date(p.periodStart).toLocaleDateString("fr-FR")} au{" "}
                    {new Date(p.periodEnd).toLocaleDateString("fr-FR")}
                  </p>
                ))}
                {reminders.map((r) => (
                  <p key={r.id} className="text-xs text-amber-900 whitespace-pre-wrap">
                    <strong>{r.label}</strong> — {r.message}
                  </p>
                ))}
              </section>
            ) : (
              <section className="rounded-xl border border-stone-200 bg-stone-50 px-4 py-3">
                <p className="text-xs text-stone-700 leading-relaxed">
                  Aucune période officielle n&apos;est associée à votre classe : vous pouvez
                  quand même déposer une demande de stage. L&apos;établissement validera ensuite.
                </p>
              </section>
            )}

            <section>
              <h2 className="text-base font-bold text-[#1F3D2B]">
                Mes stages ({dossier.conventions.length})
              </h2>
              {dossier.conventions.length === 0 ? (
                <p className="mt-2 rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 text-stone-600">
                  Aucun stage déposé pour le moment. Vous pouvez en ouvrir un ci-dessous.
                </p>
              ) : (
                <>
                  <p className="mt-2 text-xs text-emerald-800">
                    Vous avez déjà {dossier.conventions.length} dossier
                    {dossier.conventions.length > 1 ? "s" : ""} de stage cette année. Vous pouvez
                    en ouvrir un existant ou en déposer un nouveau.
                  </p>
                  <ul className="mt-3 space-y-3">
                    {dossier.conventions.map((c) => (
                      <li
                        key={c.id}
                        className="rounded-xl border border-stone-200 bg-stone-50/50 p-4 space-y-2"
                      >
                        <div className="flex flex-wrap items-start justify-between gap-2">
                          <div>
                            <p className="font-bold text-[#1F3D2B]">
                              {c.stageLabel || "Stage"}
                              {c.companyName !== "—" ? ` — ${c.companyName}` : ""}
                            </p>
                            <p className="text-xs text-stone-500 mt-0.5">
                              {formatPeriodRangeFr(c.periodStart, c.periodEnd) ||
                                `${formatIsoDateFr(c.periodStart)} → ${formatIsoDateFr(c.periodEnd)}`}{" "}
                              · {c.statusLabel}
                            </p>
                          </div>
                          <div className="flex shrink-0 flex-wrap gap-2">
                            {(c.status === "signed" || c.signatureSummary.complete) &&
                            c.studentAccessToken ? (
                              <a
                                href={`/api/stages/public/student/pdf?token=${encodeURIComponent(c.studentAccessToken)}&download=1`}
                                className="rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-white"
                                download
                              >
                                PDF
                              </a>
                            ) : null}
                            <button
                              type="button"
                              onClick={() => openExistingStage(c)}
                              className="rounded-lg border border-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-[#2F6B4A]"
                            >
                              Ouvrir
                            </button>
                          </div>
                        </div>
                        <StageSignatureProgress summary={c.signatureSummary} compact />
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </section>

            <section className="rounded-xl border border-emerald-200 bg-emerald-50/50 p-4 space-y-3">
              <h2 className="text-sm font-bold text-emerald-900">Nouveau stage</h2>
              <p className="text-xs text-emerald-900/90 leading-relaxed">
                Déposez une demande même hors période officielle. L&apos;établissement décidera
                ensuite d&apos;accepter ou de refuser.
              </p>
              {(dossier.availablePeriods.length > 0) && (
                <label className="block text-xs">
                  Période officielle concernée (optionnel)
                  <select
                    className="mt-1 w-full rounded-lg border px-3 py-2"
                    value={selectedPeriodId}
                    onChange={(e) => setSelectedPeriodId(e.target.value)}
                  >
                    <option value="">— Hors période / autre date —</option>
                    {dossier.availablePeriods.map((p) => (
                      <option key={p.id} value={p.id} disabled={p.used}>
                        {p.label} ({formatIsoDateFr(p.periodStart)} → {formatIsoDateFr(p.periodEnd)})
                        {p.used ? " — déjà utilisée" : ""}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              <button
                type="button"
                disabled={busy || !parent1Email.trim()}
                onClick={() => void createNewStage()}
                className="w-full rounded-lg bg-[#2F6B4A] py-2.5 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy ? "Création…" : "+ Déposer un nouveau stage"}
              </button>
              {!parent1Email.trim() && (
                <p className="text-xs text-rose-700">
                  Indiquez au moins l&apos;e-mail du responsable légal 1 avant de continuer.
                </p>
              )}
            </section>
          </div>
        )}

        {step === "form" && convention && (
          <>
            <div className="mt-4 flex flex-wrap items-start justify-between gap-3">
              <div className="flex min-w-0 items-center gap-3">
                <div className="h-12 w-12 shrink-0 overflow-hidden rounded-full border-2 border-white bg-[#2F6B4A]/15 shadow-sm">
                  {studentPhotoUrl && !photoFailed ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img
                      src={studentPhotoUrl}
                      alt={`${convention.student.firstName} ${convention.student.lastName}`}
                      className="h-full w-full object-cover"
                      onError={() => setPhotoFailed(true)}
                    />
                  ) : (
                    <div
                      className="flex h-full w-full items-center justify-center text-xs font-black text-[#1F3D2B]"
                      aria-hidden
                    >
                      {studentInitials(convention.student.firstName, convention.student.lastName)}
                    </div>
                  )}
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-black text-[#1F3D2B]">
                    {convention.student.firstName} {convention.student.lastName}
                  </p>
                  <p className="text-xs text-stone-600">
                    <strong>{convention.stageLabel || "Stage"}</strong> · {convention.student.className}{" "}
                    · {STAGE_CONVENTION_STATUS_LABELS[convention.status]}
                  </p>
                </div>
              </div>
              <button
                type="button"
                onClick={backToDashboard}
                className="text-xs font-semibold text-[#2F6B4A] underline"
              >
                ← Mes stages
              </button>
            </div>

            {done && (
              <div className="mt-4 space-y-3">
                <p className="rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800">
                  Préconvention envoyée à l&apos;administratif pour validation. Vous serez notifié une
                  fois la convention prête à signer.
                </p>
                <button
                  type="button"
                  onClick={backToDashboard}
                  className="w-full rounded-lg border border-[#2F6B4A] bg-white py-2.5 text-sm font-bold text-[#2F6B4A]"
                >
                  Retour à mes stages
                </button>
              </div>
            )}

            {rejectNote && !done && (
              <p className="mt-4 rounded-lg bg-amber-50 border border-amber-200 px-4 py-3 text-sm text-amber-900">
                <strong>Correction demandée :</strong> {rejectNote}
              </p>
            )}

            {!readOnly && !done && (
              <div className="mt-6 space-y-4" data-tour="stages-preconvention-form">
                <div className="rounded-xl border-2 border-rose-300 bg-rose-50 px-4 py-3 text-xs text-rose-950">
                  <p className="font-black">Convention envoyée à cette adresse</p>
                  <p className="mt-1">
                    {convention.parentSignerEmail ||
                      convention.student.parent1Email ||
                      "—"}{" "}
                    — vous confirmez cet e-mail avec un code avant de valider
                    votre préconvention.
                  </p>
                  {parentEmailVerified && (
                    <p className="mt-2 font-semibold text-emerald-800">✓ E-mail confirmé</p>
                  )}
                </div>

                <StagePreconventionForm
                  convention={convention}
                  onChange={(c) => {
                    setConvention(c);
                    setParentEmailVerified(false);
                    setShowParentCode(false);
                  }}
                  onSave={() => void save("save")}
                  onSubmit={() => void save("submit")}
                  busy={busy}
                  identityLocked={Boolean(convention.ocrMeta?.matchedEleveIne)}
                  reminders={reminders}
                  officialPeriods={officialPeriods}
                  submitLabel="Valider ma préconvention"
                />

                {showParentCode && (
                  <div className="rounded-xl border border-[#2F6B4A]/25 bg-[#f3faf6] p-4 space-y-3">
                    <div>
                      <p className="text-sm font-bold text-[#1F3D2B]">
                        Confirmez l&apos;e-mail du responsable
                      </p>
                      <p className="mt-1 text-xs text-stone-600">
                        Un e-mail de vérification a été envoyé à{" "}
                        <span className="font-semibold text-[#1F3D2B]">
                          {convention.parentSignerEmail ||
                            convention.student.parent1Email ||
                            "—"}
                        </span>
                        .
                      </p>
                    </div>
                    <StageOtpCodeInput
                      value={parentCode}
                      onChange={setParentCode}
                      disabled={busy}
                      autoFocus
                    />
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        disabled={busy || parentCode.length !== 6}
                        onClick={() => void confirmParentCode()}
                        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
                      >
                        {busy ? "Validation…" : "Valider le code et envoyer"}
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void save("submit")}
                        className="rounded-lg border border-stone-300 bg-white px-4 py-2 text-sm font-semibold text-stone-700"
                      >
                        Renvoyer le code
                      </button>
                    </div>
                  </div>
                )}

                {parentEmailVerified && !done && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void save("submit")}
                    className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
                  >
                    {busy ? "Validation…" : "Valider ma préconvention"}
                  </button>
                )}
              </div>
            )}

            {readOnly && (
              <div className="mt-6 text-sm text-stone-600 space-y-4">
                <div className="rounded-xl border border-stone-200 bg-stone-50/80 px-4 py-3 space-y-2">
                  <p>
                    <strong className="text-[#1F3D2B]">Entreprise :</strong>{" "}
                    {convention.company.name}
                  </p>
                  <p>
                    <strong className="text-[#1F3D2B]">Adresse :</strong>{" "}
                    {formatCompanyAddress(convention.company) || "—"}
                  </p>
                  <p>
                    <strong className="text-[#1F3D2B]">Période :</strong>{" "}
                    {formatPeriodRangeFr(
                      convention.schedule.periodStart,
                      convention.schedule.periodEnd,
                    ) ||
                      `${formatIsoDateFr(convention.schedule.periodStart)} → ${formatIsoDateFr(convention.schedule.periodEnd)}`}
                  </p>
                  <p>
                    <strong className="text-[#1F3D2B]">Tuteur :</strong>{" "}
                    {convention.company.tutorName}
                    {convention.company.tutorEmail ? ` — ${convention.company.tutorEmail}` : ""}
                  </p>
                </div>

                <StageSchedulePanel
                  periodLabel={
                    formatPeriodRangeFr(
                      convention.schedule.periodStart,
                      convention.schedule.periodEnd,
                    ) ||
                    `${formatIsoDateFr(convention.schedule.periodStart)} → ${formatIsoDateFr(convention.schedule.periodEnd)}`
                  }
                  days={convention.schedule.days || []}
                  mode={convention.schedule.mode}
                />

                {signatureSummary && <StageSignatureProgress summary={signatureSummary} />}

                {(convention.status === "signed" || signatureSummary?.complete) && token ? (
                  <a
                    href={`/api/stages/public/student/pdf?token=${encodeURIComponent(token)}&download=1`}
                    className="flex w-full items-center justify-center rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white hover:bg-[#275c3f]"
                    download
                  >
                    Télécharger ma convention PDF
                  </a>
                ) : null}

                {canEditTutorEmail && (
                  <div className="rounded-xl border border-amber-300 bg-amber-50 p-4 space-y-3">
                    <p className="text-sm font-bold text-amber-950">
                      Corriger l&apos;e-mail du tuteur
                    </p>
                    <p className="text-xs text-amber-900">
                      Si l&apos;adresse du tuteur a renvoyé une erreur, corrigez-la ici. Une nouvelle
                      demande de signature sera envoyée automatiquement.
                    </p>
                    <input
                      type="email"
                      className="w-full rounded-lg border border-amber-300 px-3 py-2 text-sm"
                      value={tutorEmailEdit}
                      onChange={(e) => setTutorEmailEdit(e.target.value)}
                      placeholder="tuteur@entreprise.fr"
                    />
                    <button
                      type="button"
                      disabled={busy || !tutorEmailEdit.trim()}
                      onClick={() => void saveTutorEmail()}
                      className="rounded-lg bg-amber-800 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
                    >
                      Enregistrer et relancer le tuteur
                    </button>
                  </div>
                )}
              </div>
            )}
          </>
        )}
      </div>
    </main>
  );
}

export default function StagePreconventionPublicClient() {
  return (
    <Suspense fallback={<main className="p-8">Chargement…</main>}>
      <StagePreconventionPublicContent />
    </Suspense>
  );
}

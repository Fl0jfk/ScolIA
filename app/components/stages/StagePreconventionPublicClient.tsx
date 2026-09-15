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

  const [step, setStep] = useState<"identity" | "otp" | "dashboard" | "form">(
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
  const [canRequestTutorEmailChange, setCanRequestTutorEmailChange] = useState(false);
  const [tutorEmailChangePending, setTutorEmailChangePending] = useState<{
    requestedEmail: string;
    previousEmail: string;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [reminders, setReminders] = useState<StagePeriodReminder[]>([]);
  const [officialPeriods, setOfficialPeriods] = useState<StageClassPeriod[]>([]);
  const [scheduleConstraints, setScheduleConstraints] = useState<{
    rules: import("@/app/lib/stage-constraints").StageCycleConstraints;
    blockedPeriods: import("@/app/lib/stage-constraints").StageBlockedPeriod[];
  } | null>(null);
  const [cycleLabel, setCycleLabel] = useState<string | undefined>(undefined);
  const [rejectNote, setRejectNote] = useState<string | null>(null);
  const [selectedPeriodId, setSelectedPeriodId] = useState("");
  const [signatureSummary, setSignatureSummary] = useState<
    import("@/app/lib/stage-signature-summary").StageSignatureSummary | null
  >(null);

  const [parent1Email, setParent1Email] = useState("");
  const [parent2Email, setParent2Email] = useState("");
  const [editingParentEmail, setEditingParentEmail] = useState(false);
  const [infoMsg, setInfoMsg] = useState<string | null>(null);
  const [tutorEmailEdit, setTutorEmailEdit] = useState("");
  const [identityProof, setIdentityProof] = useState<string | null>(null);
  const [otpChallengeId, setOtpChallengeId] = useState<string | null>(null);
  const [otpMaskedRecipients, setOtpMaskedRecipients] = useState<string[]>([]);
  const [identityOtpCode, setIdentityOtpCode] = useState("");
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
    identityProof?: string | null;
  }) {
    writePreconventionDeviceMemory({
      nom: (fields?.nom ?? nom).trim(),
      prenom: (fields?.prenom ?? prenom).trim(),
      dateNaissance: (fields?.dateNaissance ?? dateNaissance).trim(),
      classe: (fields?.classe ?? classe).trim() || undefined,
      identityProof:
        (fields?.identityProof !== undefined
          ? fields.identityProof
          : identityProof
        )?.trim() || undefined,
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
    setIdentityProof(null);
    setOtpChallengeId(null);
    setOtpMaskedRecipients([]);
    setIdentityOtpCode("");
    setStep("identity");
    setError(null);
    setInfoMsg(null);
  }

  function applyStageContext(ctx: unknown) {
    if (!ctx || typeof ctx !== "object") {
      setReminders([]);
      setOfficialPeriods([]);
      setScheduleConstraints(null);
      setCycleLabel(undefined);
      return;
    }
    const o = ctx as {
      reminders?: StagePeriodReminder[];
      periods?: StageClassPeriod[];
      constraints?: {
        cycle?: string;
        rules?: import("@/app/lib/stage-constraints").StageCycleConstraints;
        blockedPeriods?: import("@/app/lib/stage-constraints").StageBlockedPeriod[];
      };
    };
    setReminders(Array.isArray(o.reminders) ? o.reminders : []);
    setOfficialPeriods(Array.isArray(o.periods) ? o.periods : []);
    if (o.constraints?.rules) {
      setScheduleConstraints({
        rules: o.constraints.rules,
        blockedPeriods: Array.isArray(o.constraints.blockedPeriods)
          ? o.constraints.blockedPeriods
          : [],
      });
      const cycle = o.constraints.cycle;
      setCycleLabel(
        cycle === "college"
          ? "Collège"
          : cycle === "lycee"
            ? "Lycée"
            : cycle === "ecole"
              ? "École"
              : undefined,
      );
    } else {
      setScheduleConstraints(null);
      setCycleLabel(undefined);
    }
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
    setCanRequestTutorEmailChange(data.canRequestTutorEmailChange === true);
    const pendingReq = data.tutorEmailChangeRequest ?? data.convention?.tutorEmailChangeRequest;
    setTutorEmailChangePending(
      pendingReq && typeof pendingReq === "object" && typeof pendingReq.requestedEmail === "string"
        ? {
            requestedEmail: String(pendingReq.requestedEmail),
            previousEmail: String(pendingReq.previousEmail ?? ""),
          }
        : null,
    );
    setSignatureSummary(data.signatureSummary ?? null);
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
    void identifyWithCredentials({
      ...memory,
      identityProof: memory.identityProof,
    })
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
      identityProof: identityProof || memory?.identityProof || undefined,
      ...extra,
    };
  }

  function applyIdentifySuccess(
    data: {
      studentPreview: StudentPreview;
      dossier: StudentDossier;
      stageContext?: unknown;
      identityProof?: string;
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
    const proof = String(data.identityProof ?? identityProof ?? "").trim() || null;
    if (proof) setIdentityProof(proof);
    setOtpChallengeId(null);
    setOtpMaskedRecipients([]);
    setIdentityOtpCode("");
    rememberIdentity({
      nom: (identity?.nom || preview.lastName || nom).trim(),
      prenom: (identity?.prenom || preview.firstName || prenom).trim(),
      dateNaissance: (identity?.dateNaissance || dateNaissance).trim(),
      classe: nextClasse || undefined,
      identityProof: proof,
    });
    setStep("dashboard");
  }

  function enterOtpStep(data: {
    challengeId: string;
    maskedRecipients?: string[];
    message?: string;
  }) {
    setOtpChallengeId(String(data.challengeId ?? "").trim() || null);
    setOtpMaskedRecipients(
      Array.isArray(data.maskedRecipients)
        ? data.maskedRecipients.map((r) => String(r)).filter(Boolean)
        : [],
    );
    setIdentityOtpCode("");
    setStep("otp");
    setInfoMsg(
      data.message ||
        "Le code a été envoyé. Vérifiez aussi vos spams / courriers indésirables.",
    );
  }

  async function identifyWithCredentials(creds: {
    nom: string;
    prenom: string;
    dateNaissance: string;
    classe?: string;
    identityProof?: string;
  }) {
    const res = await fetch("/api/stages/public/preconvention", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        nom: creds.nom.trim(),
        prenom: creds.prenom.trim(),
        dateNaissance: creds.dateNaissance,
        classe: creds.classe?.trim() || undefined,
        identityProof: creds.identityProof?.trim() || undefined,
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

    if (data?.needsOtp === true && data.challengeId) {
      enterOtpStep(data);
      return { ok: true as const, needsOtp: true as const };
    }

    applyIdentifySuccess(data, creds);
    return { ok: true as const };
  }

  async function confirmIdentityOtp(e?: React.FormEvent) {
    e?.preventDefault();
    if (!otpChallengeId || identityOtpCode.length !== 6) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "confirm_identity_otp",
          challengeId: otpChallengeId,
          code: identityOtpCode,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Code invalide");
      applyIdentifySuccess(data, {
        nom: nom.trim(),
        prenom: prenom.trim(),
        dateNaissance,
        classe: classe.trim() || undefined,
      });
      setInfoMsg(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function resendIdentityOtp() {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(
          identityPayload({
            action: "resend_identity_otp",
          }),
        ),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Renvoi impossible");
      enterOtpStep(data);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
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

  async function requestTutorEmailChange() {
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
          action: "request_tutor_email_change",
          tutorEmail: tutorEmailEdit,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setConvention(data.convention);
      setCanRequestTutorEmailChange(false);
      setTutorEmailChangePending({
        requestedEmail: String(data.convention?.tutorEmailChangeRequest?.requestedEmail ?? tutorEmailEdit),
        previousEmail: String(
          data.convention?.tutorEmailChangeRequest?.previousEmail ??
            convention?.company.tutorEmail ??
            "",
        ),
      });
      setInfoMsg(
        data.message ||
          "Demande enregistrée. L'établissement doit la valider avant de relancer le tuteur.",
      );
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
    router.replace("/stages/preconvention");
    const memory = readPreconventionDeviceMemory();
    const creds = {
      nom: nom.trim() || memory?.nom || "",
      prenom: prenom.trim() || memory?.prenom || "",
      dateNaissance: dateNaissance || memory?.dateNaissance || "",
      classe: classe.trim() || memory?.classe || undefined,
      identityProof: identityProof || memory?.identityProof || undefined,
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
              {busy ? "Vérification…" : "Recevoir le code d'accès →"}
            </button>
          </form>
        )}

        {step === "otp" && !token && (
          <form onSubmit={(e) => void confirmIdentityOtp(e)} className="mt-6 space-y-4 text-sm">
            <div className="rounded-xl border border-emerald-200 bg-emerald-50/80 px-4 py-3 space-y-2">
              <p className="text-sm font-bold text-[#1F3D2B]">Le code a été envoyé</p>
              <p className="text-xs text-stone-700 leading-relaxed">
                Un même code à 6 chiffres a été envoyé à toutes les adresses connues (élève et
                responsables). Saisissez-le ci-dessous pour accéder à vos stages.
              </p>
              {otpMaskedRecipients.length > 0 && (
                <ul className="text-xs text-[#1F3D2B] space-y-1">
                  {otpMaskedRecipients.map((addr) => (
                    <li key={addr} className="font-mono font-semibold">
                      → {addr}
                    </li>
                  ))}
                </ul>
              )}
              <p className="text-xs font-semibold text-amber-900 bg-amber-50 border border-amber-200 rounded-lg px-2.5 py-1.5">
                Pensez à vérifier vos spams / courriers indésirables — le message y arrive
                souvent.
              </p>
            </div>
            <StageOtpCodeInput
              value={identityOtpCode}
              onChange={setIdentityOtpCode}
              disabled={busy}
              autoFocus
            />
            <button
              type="submit"
              disabled={busy || identityOtpCode.length !== 6}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Validation…" : "Valider le code →"}
            </button>
            <div className="flex flex-wrap gap-3 justify-between">
              <button
                type="button"
                disabled={busy}
                onClick={() => void resendIdentityOtp()}
                className="text-xs font-semibold text-[#2F6B4A] underline"
              >
                Renvoyer le code
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setOtpChallengeId(null);
                  setOtpMaskedRecipients([]);
                  setIdentityOtpCode("");
                  setInfoMsg(null);
                  setStep("identity");
                }}
                className="text-xs font-semibold text-stone-600 underline"
              >
                ← Modifier l&apos;identité
              </button>
            </div>
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
                <div className="rounded-xl border border-stone-200 bg-stone-50 px-4 py-3 text-xs text-stone-700">
                  <p className="font-bold text-[#1F3D2B]">Responsable(s) qui signeront</p>
                  <p className="mt-1">
                    La convention sera proposée à{" "}
                    <span className="font-semibold">
                      {convention.parentSignerEmail ||
                        convention.student.parent1Email ||
                        "—"}
                    </span>
                    {(convention.parent2SignerEmail || convention.student.parent2Email) && (
                      <>
                        {" "}
                        et{" "}
                        <span className="font-semibold">
                          {convention.parent2SignerEmail || convention.student.parent2Email}
                        </span>
                      </>
                    )}
                    . Vous pouvez corriger ces adresses dans le formulaire ci-dessous.
                  </p>
                </div>

                <StagePreconventionForm
                  convention={convention}
                  onChange={(c) => {
                    setConvention(c);
                  }}
                  onSave={() => void save("save")}
                  onSubmit={() => void save("submit")}
                  busy={busy}
                  identityLocked={Boolean(convention.ocrMeta?.matchedEleveIne)}
                  reminders={reminders}
                  officialPeriods={officialPeriods}
                  scheduleConstraints={scheduleConstraints}
                  cycleLabel={cycleLabel}
                  submitLabel="Valider ma préconvention"
                />
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

                {tutorEmailChangePending && (
                  <div className="rounded-xl border border-amber-300 bg-amber-50 p-4 space-y-2">
                    <p className="text-sm font-bold text-amber-950">
                      Demande de changement d&apos;e-mail tuteur en cours
                    </p>
                    <p className="text-xs text-amber-900 leading-relaxed">
                      Vous avez demandé de remplacer{" "}
                      <strong>{tutorEmailChangePending.previousEmail || "—"}</strong> par{" "}
                      <strong>{tutorEmailChangePending.requestedEmail}</strong>. L&apos;établissement
                      doit valider cette demande avant toute relance.
                    </p>
                  </div>
                )}

                {canRequestTutorEmailChange && !tutorEmailChangePending && (
                  <div className="rounded-xl border border-amber-300 bg-amber-50 p-4 space-y-3">
                    <p className="text-sm font-bold text-amber-950">
                      Demander un changement d&apos;e-mail du tuteur
                    </p>
                    <p className="text-xs text-amber-900 leading-relaxed">
                      Si l&apos;adresse est erronée, proposez-en une autre. L&apos;établissement
                      validera avant d&apos;envoyer une nouvelle demande de signature — le
                      changement n&apos;est pas immédiat.
                    </p>
                    <input
                      type="email"
                      className="w-full rounded-lg border border-amber-300 px-3 py-2 text-sm"
                      value={tutorEmailEdit}
                      onChange={(e) => setTutorEmailEdit(e.target.value)}
                      placeholder="nouveau.tuteur@entreprise.fr"
                    />
                    <button
                      type="button"
                      disabled={busy || !tutorEmailEdit.trim()}
                      onClick={() => void requestTutorEmailChange()}
                      className="rounded-lg bg-amber-800 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
                    >
                      Envoyer la demande à l&apos;établissement
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

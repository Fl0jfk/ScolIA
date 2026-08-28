"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type { StageConvention } from "@/app/lib/stage-types";
import type { StageClassPeriod, StagePeriodReminder } from "@/app/lib/stage-periods-config";
import type { StageConventionCard } from "@/app/lib/stage-signature-summary";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";

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
};

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

  const [step, setStep] = useState<"identity" | "dashboard" | "form">(
    tokenFromUrl ? "form" : "identity",
  );
  const [ine, setIne] = useState("");
  const [dateNaissance, setDateNaissance] = useState("");
  const [studentPreview, setStudentPreview] = useState<StudentPreview | null>(null);
  const [dossier, setDossier] = useState<StudentDossier | null>(null);
  const [token, setToken] = useState(tokenFromUrl);
  const [convention, setConvention] = useState<StageConvention | null>(null);
  const [readOnly, setReadOnly] = useState(false);
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
    setSignatureSummary(data.signatureSummary ?? null);
    applyStageContext(data.stageContext);
    if (data.convention?.status === "admin_rejected" && data.convention.adminReview?.note) {
      setRejectNote(data.convention.adminReview.note);
    } else {
      setRejectNote(null);
    }
    setStep("form");
  }, []);

  useEffect(() => {
    if (tokenFromUrl) {
      setToken(tokenFromUrl);
      void loadConvention(tokenFromUrl).catch((e: unknown) =>
        setError(e instanceof Error ? e.message : "Erreur"),
      );
    }
  }, [tokenFromUrl, loadConvention]);

  async function verifyIdentity(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ine, dateNaissance, action: "identify" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");

      setStudentPreview(data.studentPreview);
      setDossier(data.dossier);
      applyStageContext(data.stageContext);
      setStep("dashboard");
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
        body: JSON.stringify({
          ine,
          dateNaissance,
          action: "create",
          periodId: selectedPeriodId || undefined,
        }),
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

  function backToDashboard() {
    setToken("");
    setConvention(null);
    setDone(false);
    setRejectNote(null);
    router.replace("/stages/preconvention");
    if (studentPreview && dossier) {
      setStep("dashboard");
      void (async () => {
        const res = await fetch("/api/stages/public/preconvention", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ ine, dateNaissance, action: "identify" }),
        });
        const data = await res.json();
        if (res.ok) {
          setDossier(data.dossier);
          applyStageContext(data.stageContext);
        }
      })();
    } else {
      setStep("identity");
    }
  }

  const freePeriods = dossier?.availablePeriods.filter((p) => !p.used) ?? [];

  if (step === "form" && !convention && !error) {
    return <main className="min-h-screen flex items-center justify-center p-6">Chargement…</main>;
  }

  return (
    <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
      <div className="mx-auto max-w-2xl rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h1 className="text-2xl font-black text-[#1F3D2B]">Préconvention de stage</h1>
        <p className="mt-2 text-sm text-stone-600">
          Vous pouvez déposer plusieurs stages dans l&apos;année (ex. deux semaines en deux
          entreprises différentes). Chaque stage a son propre suivi de signatures.
        </p>

        {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}

        {step === "identity" && !token && (
          <form onSubmit={(e) => void verifyIdentity(e)} className="mt-6 space-y-4 text-sm">
            <p className="rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 text-xs text-stone-700">
              <strong>Étape 1 — Identification</strong> : INE et date de naissance (bulletin /
              Pronote). Vous accéderez ensuite à vos dossiers de stage.
            </p>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">
                Identifiant national élève (INE) *
              </span>
              <input
                className="mt-1 w-full rounded-lg border px-3 py-2 font-mono uppercase"
                placeholder="ex. 180123456AB"
                value={ine}
                onChange={(e) => setIne(e.target.value.toUpperCase())}
                autoComplete="off"
                required
              />
            </label>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">Date de naissance *</span>
              <input
                type="date"
                className="mt-1 w-full rounded-lg border px-3 py-2"
                value={dateNaissance}
                onChange={(e) => setDateNaissance(e.target.value)}
                required
              />
            </label>
            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Vérification…" : "Accéder à mes stages →"}
            </button>
          </form>
        )}

        {step === "dashboard" && studentPreview && dossier && (
          <div className="mt-6 space-y-6 text-sm">
            <p className="text-stone-600">
              <strong>
                {studentPreview.firstName} {studentPreview.lastName}
              </strong>{" "}
              · {studentPreview.className} · Année {dossier.schoolYear}
            </p>

            {(reminders.length > 0 || officialPeriods.length > 0) && (
              <section className="rounded-xl border border-amber-200 bg-amber-50/80 p-4 space-y-2">
                <h2 className="text-sm font-bold text-amber-900">Rappels — dates de stage</h2>
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
            )}

            <section>
              <h2 className="text-base font-bold text-[#1F3D2B]">Mes stages ({dossier.conventions.length})</h2>
              {dossier.conventions.length === 0 ? (
                <p className="mt-2 text-stone-500">Aucun stage déposé pour le moment.</p>
              ) : (
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
                            {c.periodStart} → {c.periodEnd} · {c.statusLabel}
                          </p>
                        </div>
                        <button
                          type="button"
                          onClick={() => openExistingStage(c)}
                          className="shrink-0 rounded-lg border border-[#2F6B4A] px-3 py-1.5 text-xs font-semibold text-[#2F6B4A]"
                        >
                          Ouvrir
                        </button>
                      </div>
                      <StageSignatureProgress summary={c.signatureSummary} compact />
                    </li>
                  ))}
                </ul>
              )}
            </section>

            {dossier.canCreateNew && (
              <section className="rounded-xl border border-emerald-200 bg-emerald-50/50 p-4 space-y-3">
                <h2 className="text-sm font-bold text-emerald-900">Nouveau stage</h2>
                {freePeriods.length > 0 && (
                  <label className="block text-xs">
                    Période concernée (optionnel)
                    <select
                      className="mt-1 w-full rounded-lg border px-3 py-2"
                      value={selectedPeriodId}
                      onChange={(e) => setSelectedPeriodId(e.target.value)}
                    >
                      <option value="">— Choisir une période —</option>
                      {freePeriods.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.label} ({p.periodStart} → {p.periodEnd})
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void createNewStage()}
                  className="w-full rounded-lg bg-[#2F6B4A] py-2.5 text-sm font-bold text-white disabled:opacity-50"
                >
                  {busy ? "Création…" : "+ Déposer un nouveau stage"}
                </button>
              </section>
            )}
          </div>
        )}

        {step === "form" && convention && (
          <>
            <div className="mt-4 flex flex-wrap items-center justify-between gap-2">
              <p className="text-sm text-stone-600">
                <strong>
                  {convention.stageLabel || "Stage"}
                </strong>{" "}
                · {convention.student.firstName} {convention.student.lastName} (
                {convention.student.className}) · {STAGE_CONVENTION_STATUS_LABELS[convention.status]}
              </p>
              <button
                type="button"
                onClick={backToDashboard}
                className="text-xs font-semibold text-[#2F6B4A] underline"
              >
                ← Mes stages
              </button>
            </div>

            {done && (
              <p className="mt-4 rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800">
                Préconvention envoyée à l&apos;administratif pour validation. Vous serez notifié une
                fois la convention prête à signer.
              </p>
            )}

            {rejectNote && !done && (
              <p className="mt-4 rounded-lg bg-amber-50 border border-amber-200 px-4 py-3 text-sm text-amber-900">
                <strong>Correction demandée :</strong> {rejectNote}
              </p>
            )}

            {!readOnly && !done && (
              <div className="mt-6" data-tour="stages-preconvention-form">
                <StagePreconventionForm
                  convention={convention}
                  onChange={setConvention}
                  onSave={() => void save("save")}
                  onSubmit={() => void save("submit")}
                  busy={busy}
                  identityLocked={Boolean(convention.ocrMeta?.matchedEleveIne)}
                  reminders={reminders}
                  officialPeriods={officialPeriods}
                />
              </div>
            )}

            {readOnly && (
              <div className="mt-6 text-sm text-stone-600 space-y-4">
                <div className="space-y-2">
                  <p>
                    <strong>Entreprise :</strong> {convention.company.name}
                  </p>
                  <p>
                    <strong>Période :</strong> {convention.schedule.periodStart} →{" "}
                    {convention.schedule.periodEnd}
                  </p>
                  <p>
                    <strong>Horaires :</strong> {scheduleSummary(convention.schedule)}
                  </p>
                </div>
                {signatureSummary && <StageSignatureProgress summary={signatureSummary} />}
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

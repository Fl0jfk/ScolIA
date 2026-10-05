"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { StageInternshipKind } from "@/app/lib/stage-types";

type StudentPreset = {
  firstName: string;
  lastName: string;
  className: string;
  dateNaissance?: string;
  ine?: string;
  email?: string;
};

type ClassRosterLite = {
  availableClasses: string[];
  roster: {
    className: string;
    students: Array<{
      nom: string;
      prenom: string;
      ine?: string;
      photoUrl?: string | null;
    }>;
  } | null;
};

const INTERNSHIP_OPTIONS: Array<{ value: StageInternshipKind; label: string }> = [
  { value: "stage_observation", label: "Stage d'observation" },
  { value: "pfmp", label: "PFMP / Stage en entreprise" },
  { value: "job_ete", label: "Job d'été" },
  { value: "autre", label: "Autre" },
];

const fieldCls =
  "w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm text-stone-900 shadow-sm focus:border-[#2F6B4A] focus:outline-none focus:ring-2 focus:ring-[#2F6B4A]/20";

export default function StageOfflineCreateModal({
  open,
  onClose,
  onCreated,
  presetStudent,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: (conventionId: string) => void;
  presetStudent?: StudentPreset | null;
}) {
  const [classes, setClasses] = useState<string[]>([]);
  const [className, setClassName] = useState(presetStudent?.className || "");
  const [students, setStudents] = useState<
    Array<{ nom: string; prenom: string; ine?: string; photoUrl?: string | null }>
  >([]);
  const [studentKey, setStudentKey] = useState("");
  const [firstName, setFirstName] = useState(presetStudent?.firstName || "");
  const [lastName, setLastName] = useState(presetStudent?.lastName || "");
  const [dateNaissance, setDateNaissance] = useState(presetStudent?.dateNaissance || "");
  const [ine, setIne] = useState(presetStudent?.ine || "");
  const [companyName, setCompanyName] = useState("");
  const [companyAddress, setCompanyAddress] = useState("");
  const [companyPostalCode, setCompanyPostalCode] = useState("");
  const [companyCity, setCompanyCity] = useState("");
  const [companySiret, setCompanySiret] = useState("");
  const [companyActivity, setCompanyActivity] = useState("");
  const [tutorName, setTutorName] = useState("");
  const [tutorEmail, setTutorEmail] = useState("");
  const [tutorPhone, setTutorPhone] = useState("");
  const [periodStart, setPeriodStart] = useState("");
  const [periodEnd, setPeriodEnd] = useState("");
  const [internshipKind, setInternshipKind] = useState<StageInternshipKind>("stage_observation");
  const [stageLabel, setStageLabel] = useState("");
  const [note, setNote] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loadingRoster, setLoadingRoster] = useState(false);

  const resetForm = useCallback((preset?: StudentPreset | null) => {
    setClassName(preset?.className || "");
    setStudentKey("");
    setFirstName(preset?.firstName || "");
    setLastName(preset?.lastName || "");
    setDateNaissance(preset?.dateNaissance || "");
    setIne(preset?.ine || "");
    setCompanyName("");
    setCompanyAddress("");
    setCompanyPostalCode("");
    setCompanyCity("");
    setCompanySiret("");
    setCompanyActivity("");
    setTutorName("");
    setTutorEmail("");
    setTutorPhone("");
    setPeriodStart("");
    setPeriodEnd("");
    setInternshipKind("stage_observation");
    setStageLabel("");
    setNote("");
    setFile(null);
    setError(null);
  }, []);

  useEffect(() => {
    if (!open) return;
    resetForm(presetStudent);
  }, [open, presetStudent, resetForm]);

  const loadClasses = useCallback(async () => {
    try {
      const res = await fetch("/api/stages/class-roster", { cache: "no-store" });
      const data = (await res.json()) as ClassRosterLite & { error?: string };
      if (!res.ok) throw new Error(data.error || "Erreur");
      setClasses(data.availableClasses || []);
    } catch {
      /* classes optionnelles — saisie manuelle possible */
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    void loadClasses();
  }, [open, loadClasses]);

  useEffect(() => {
    if (!open) return;
    if (presetStudent?.className) {
      setClassName(presetStudent.className);
    }
  }, [open, presetStudent?.className]);

  const loadStudents = useCallback(async (cls: string) => {
    if (!cls.trim()) {
      setStudents([]);
      return;
    }
    setLoadingRoster(true);
    try {
      const params = new URLSearchParams({ className: cls });
      const res = await fetch(`/api/stages/class-roster?${params}`, { cache: "no-store" });
      const data = (await res.json()) as ClassRosterLite & { error?: string };
      if (!res.ok) throw new Error(data.error || "Erreur");
      setStudents(data.roster?.students || []);
    } catch {
      setStudents([]);
    } finally {
      setLoadingRoster(false);
    }
  }, []);

  useEffect(() => {
    if (!open || !className) return;
    void loadStudents(className);
  }, [open, className, loadStudents]);

  const studentOptions = useMemo(
    () =>
      students.map((s) => ({
        key: `${s.nom}|${s.prenom}|${s.ine || ""}`,
        label: `${s.prenom} ${s.nom}`,
        prenom: s.prenom,
        nom: s.nom,
        ine: s.ine,
      })),
    [students],
  );

  function applyStudent(key: string) {
    setStudentKey(key);
    const found = studentOptions.find((s) => s.key === key);
    if (!found) return;
    setFirstName(found.prenom);
    setLastName(found.nom);
    setIne(found.ine || "");
  }

  async function submit() {
    if (!file) {
      setError("Déposez le PDF de la convention déjà signée.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const fd = new FormData();
      fd.set("file", file);
      fd.set("studentFirstName", firstName);
      fd.set("studentLastName", lastName);
      fd.set("studentClassName", className);
      if (dateNaissance) fd.set("studentDateNaissance", dateNaissance);
      if (ine) fd.set("matchedEleveIne", ine);
      fd.set("companyName", companyName);
      fd.set("companyAddress", companyAddress);
      if (companyPostalCode) fd.set("companyPostalCode", companyPostalCode);
      if (companyCity) fd.set("companyCity", companyCity);
      if (companySiret) fd.set("companySiret", companySiret);
      if (companyActivity) fd.set("companyActivity", companyActivity);
      if (tutorName) fd.set("tutorName", tutorName);
      if (tutorEmail) fd.set("tutorEmail", tutorEmail);
      if (tutorPhone) fd.set("tutorPhone", tutorPhone);
      fd.set("periodStart", periodStart);
      fd.set("periodEnd", periodEnd);
      fd.set("internshipKind", internshipKind);
      if (stageLabel.trim()) fd.set("stageLabel", stageLabel.trim());
      if (note.trim()) fd.set("note", note.trim());

      const res = await fetch("/api/stages/conventions/offline", {
        method: "POST",
        body: fd,
      });
      const data = (await res.json()) as {
        error?: string;
        convention?: { id: string };
      };
      if (!res.ok) throw new Error(data.error || "Erreur");
      if (!data.convention?.id) throw new Error("Réponse invalide.");
      onCreated(data.convention.id);
      onClose();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-end justify-center bg-black/40 p-3 sm:items-center sm:p-6">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="offline-stage-title"
        className="max-h-[92vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-stone-200 bg-white shadow-xl"
      >
        <div className="sticky top-0 z-10 flex items-start justify-between gap-3 border-b border-stone-100 bg-white px-5 py-4">
          <div>
            <h2 id="offline-stage-title" className="text-lg font-bold text-[#1F3D2B]">
              Stage hors plateforme
            </h2>
            <p className="mt-1 text-sm text-stone-600">
              Convention déjà signée sur papier — enregistrement sans circuit de signatures
              ScolIA.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg border border-stone-200 px-2.5 py-1 text-sm font-semibold text-stone-600 hover:bg-stone-50"
          >
            Fermer
          </button>
        </div>

        <div className="space-y-6 px-5 py-5">
          {error && (
            <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
              {error}
            </p>
          )}

          <section className="space-y-3">
            <h3 className="text-sm font-bold text-[#1F3D2B]">1. Élève</h3>
            <label className="block text-xs font-semibold text-stone-600">
              Classe *
              {classes.length > 0 ? (
                <select
                  className={`${fieldCls} mt-1`}
                  value={classes.includes(className) ? className : ""}
                  onChange={(e) => {
                    setClassName(e.target.value);
                    setStudentKey("");
                  }}
                >
                  <option value="">— Choisir ou saisir ci-dessous —</option>
                  {classes.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              ) : null}
              <input
                className={`${fieldCls} mt-1`}
                value={className}
                onChange={(e) => {
                  setClassName(e.target.value);
                  setStudentKey("");
                }}
                placeholder="ex. 3A"
              />
            </label>
            {className ? (
              <label className="block text-xs font-semibold text-stone-600">
                Élève de la classe
                <select
                  className={`${fieldCls} mt-1`}
                  value={studentKey}
                  disabled={loadingRoster}
                  onChange={(e) => applyStudent(e.target.value)}
                >
                  <option value="">
                    {loadingRoster ? "Chargement…" : "— Sélectionner ou saisir ci-dessous —"}
                  </option>
                  {studentOptions.map((s) => (
                    <option key={s.key} value={s.key}>
                      {s.label}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-xs font-semibold text-stone-600">
                Prénom *
                <input
                  className={`${fieldCls} mt-1`}
                  value={firstName}
                  onChange={(e) => setFirstName(e.target.value)}
                />
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                Nom *
                <input
                  className={`${fieldCls} mt-1`}
                  value={lastName}
                  onChange={(e) => setLastName(e.target.value)}
                />
              </label>
            </div>
          </section>

          <section className="space-y-3">
            <h3 className="text-sm font-bold text-[#1F3D2B]">2. Entreprise & période</h3>
            <label className="block text-xs font-semibold text-stone-600">
              Raison sociale *
              <input
                className={`${fieldCls} mt-1`}
                value={companyName}
                onChange={(e) => setCompanyName(e.target.value)}
              />
            </label>
            <label className="block text-xs font-semibold text-stone-600">
              Adresse *
              <input
                className={`${fieldCls} mt-1`}
                value={companyAddress}
                onChange={(e) => setCompanyAddress(e.target.value)}
                placeholder="12 rue de la République"
              />
            </label>
            <div className="grid gap-3 sm:grid-cols-[8rem_1fr]">
              <label className="block text-xs font-semibold text-stone-600">
                Code postal
                <input
                  className={`${fieldCls} mt-1`}
                  value={companyPostalCode}
                  onChange={(e) =>
                    setCompanyPostalCode(e.target.value.replace(/\D/g, "").slice(0, 5))
                  }
                  inputMode="numeric"
                  maxLength={5}
                />
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                Ville
                <input
                  className={`${fieldCls} mt-1`}
                  value={companyCity}
                  onChange={(e) => setCompanyCity(e.target.value)}
                />
              </label>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-xs font-semibold text-stone-600">
                SIRET
                <input
                  className={`${fieldCls} mt-1`}
                  value={companySiret}
                  onChange={(e) => setCompanySiret(e.target.value)}
                />
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                Activité
                <input
                  className={`${fieldCls} mt-1`}
                  value={companyActivity}
                  onChange={(e) => setCompanyActivity(e.target.value)}
                />
              </label>
            </div>
            <div className="grid gap-3 sm:grid-cols-3">
              <label className="block text-xs font-semibold text-stone-600">
                Tuteur
                <input
                  className={`${fieldCls} mt-1`}
                  value={tutorName}
                  onChange={(e) => setTutorName(e.target.value)}
                />
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                E-mail tuteur
                <input
                  className={`${fieldCls} mt-1`}
                  type="email"
                  value={tutorEmail}
                  onChange={(e) => setTutorEmail(e.target.value)}
                />
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                Téléphone
                <input
                  className={`${fieldCls} mt-1`}
                  value={tutorPhone}
                  onChange={(e) => setTutorPhone(e.target.value)}
                />
              </label>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-xs font-semibold text-stone-600">
                Début du stage *
                <input
                  className={`${fieldCls} mt-1`}
                  type="date"
                  value={periodStart}
                  onChange={(e) => setPeriodStart(e.target.value)}
                />
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                Fin du stage *
                <input
                  className={`${fieldCls} mt-1`}
                  type="date"
                  value={periodEnd}
                  onChange={(e) => setPeriodEnd(e.target.value)}
                />
              </label>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-xs font-semibold text-stone-600">
                Type de stage
                <select
                  className={`${fieldCls} mt-1`}
                  value={internshipKind}
                  onChange={(e) => setInternshipKind(e.target.value as StageInternshipKind)}
                >
                  {INTERNSHIP_OPTIONS.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block text-xs font-semibold text-stone-600">
                Libellé (optionnel)
                <input
                  className={`${fieldCls} mt-1`}
                  value={stageLabel}
                  onChange={(e) => setStageLabel(e.target.value)}
                  placeholder="ex. PFMP 1"
                />
              </label>
            </div>
          </section>

          <section className="space-y-3">
            <h3 className="text-sm font-bold text-[#1F3D2B]">3. PDF déjà signé</h3>
            <label className="flex cursor-pointer flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed border-stone-300 bg-stone-50 px-4 py-8 text-center hover:border-[#2F6B4A]/50 hover:bg-[#f6faf8]">
              <span className="text-sm font-semibold text-[#1F3D2B]">
                {file ? file.name : "Choisir ou déposer le PDF"}
              </span>
              <span className="text-xs text-stone-500">PDF uniquement — max 15 Mo</span>
              <input
                type="file"
                accept="application/pdf,.pdf"
                className="sr-only"
                onChange={(e) => setFile(e.target.files?.[0] || null)}
              />
            </label>
            <label className="block text-xs font-semibold text-stone-600">
              Note interne (optionnelle)
              <textarea
                className={`${fieldCls} mt-1 min-h-[4rem]`}
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder="ex. Convention reçue par courrier le 12/03"
              />
            </label>
          </section>
        </div>

        <div className="sticky bottom-0 flex flex-wrap items-center justify-end gap-2 border-t border-stone-100 bg-white px-5 py-4">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="rounded-lg border border-stone-300 bg-white px-4 py-2 text-sm font-semibold text-stone-700 disabled:opacity-50"
          >
            Annuler
          </button>
          <button
            type="button"
            onClick={() => void submit()}
            disabled={busy}
            className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-bold text-white shadow-sm hover:bg-[#275a3e] disabled:opacity-50"
          >
            {busy ? "Enregistrement…" : "Enregistrer le stage"}
          </button>
        </div>
      </div>
    </div>
  );
}

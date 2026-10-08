"use client";

import { useEffect, useState } from "react";
import DirectoryPersonSelect, {
  DirectoryPeopleSelect,
  DirectoryPeoplePersonSelect,
  directoryMemberLabel,
} from "@/app/components/settings/DirectoryPersonSelect";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import type { AbsenceNotifyPerson } from "@/app/lib/app-config-schemas";

type ProcessorsPayload = {
  absencesValidatorsOgec: AbsenceNotifyPerson[];
  absencesValidatorsProfEcole: AbsenceNotifyPerson[];
  absencesValidatorsProfCollege: AbsenceNotifyPerson[];
  absencesValidatorsProfLycee: AbsenceNotifyPerson[];
  absencesNotifyProfEcole: AbsenceNotifyPerson | null;
  absencesNotifyProfCollege: AbsenceNotifyPerson | null;
  absencesNotifyProfLycee: AbsenceNotifyPerson | null;
  absencesNotifyOgecCompta: string[];
};

const emptyPayload = (): ProcessorsPayload => ({
  absencesValidatorsOgec: [],
  absencesValidatorsProfEcole: [],
  absencesValidatorsProfCollege: [],
  absencesValidatorsProfLycee: [],
  absencesNotifyProfEcole: null,
  absencesNotifyProfCollege: null,
  absencesNotifyProfLycee: null,
  absencesNotifyOgecCompta: [],
});

export default function AbsencesProcessorsPanel() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [members, setMembers] = useState<DirectoryMemberOption[]>([]);
  const [processors, setProcessors] = useState<ProcessorsPayload>(emptyPayload);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/absences/processors", { cache: "no-store" });
        const data = (await res.json().catch(() => null)) as {
          error?: string;
          processors?: Partial<ProcessorsPayload>;
          members?: DirectoryMemberOption[];
          viewerCanConfigure?: boolean;
        } | null;
        if (!res.ok || !data) throw new Error(data?.error || "Chargement impossible");
        if (!data.viewerCanConfigure) {
          throw new Error("Paramétrage réservé à la direction.");
        }
        if (!cancelled) {
          setProcessors({
            absencesValidatorsOgec: data.processors?.absencesValidatorsOgec ?? [],
            absencesValidatorsProfEcole: data.processors?.absencesValidatorsProfEcole ?? [],
            absencesValidatorsProfCollege: data.processors?.absencesValidatorsProfCollege ?? [],
            absencesValidatorsProfLycee: data.processors?.absencesValidatorsProfLycee ?? [],
            absencesNotifyProfEcole: data.processors?.absencesNotifyProfEcole ?? null,
            absencesNotifyProfCollege: data.processors?.absencesNotifyProfCollege ?? null,
            absencesNotifyProfLycee: data.processors?.absencesNotifyProfLycee ?? null,
            absencesNotifyOgecCompta: data.processors?.absencesNotifyOgecCompta ?? [],
          });
          setMembers(data.members || []);
        }
      } catch (e: unknown) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Erreur");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const setTreatPerson = (
    key: "absencesNotifyProfEcole" | "absencesNotifyProfCollege" | "absencesNotifyProfLycee",
    member: DirectoryMemberOption | null,
  ) => {
    setProcessors((p) => ({
      ...p,
      [key]: member
        ? {
            label: directoryMemberLabel(member),
            email: member.email.trim(),
            userId: member.externalUserId,
          }
        : null,
    }));
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/absences/processors", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(processors),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible");
      setMessage("Paramétrage enregistré.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="rounded-3xl border border-slate-200 bg-white p-8 text-slate-500">
        Chargement…
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <section className="space-y-3">
        <div className="rounded-3xl border border-amber-200 bg-amber-50/60 p-5">
          <p className="text-[11px] font-black uppercase tracking-widest text-amber-800">
            1 · Validation
          </p>
          <h3 className="mt-1 font-black text-slate-900">Qui accepte / prend acte ?</h3>
          <p className="mt-1 text-sm text-slate-600">
            File « Absences à valider ». Arrêt maladie / enfant malade / congé exceptionnel : la
            direction prend acte (pas de refus), puis le dossier part en traitement. Liste vide =
            directeur de l&apos;établissement concerné (Paramètres → Établissements).
          </p>
        </div>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Personnel OGEC — validateurs par défaut
          </span>
          <p className="mt-1 text-xs text-slate-500">
            Exceptions individuelles : fiche RH du personnel → « Absences — qui valide ? ».
          </p>
          <div className="mt-2">
            <DirectoryPeoplePersonSelect
              members={members}
              selected={processors.absencesValidatorsOgec}
              onChange={(people) =>
                setProcessors((p) => ({ ...p, absencesValidatorsOgec: people }))
              }
            />
          </div>
        </label>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Professeurs — école (validation)
          </span>
          <div className="mt-2">
            <DirectoryPeoplePersonSelect
              members={members}
              selected={processors.absencesValidatorsProfEcole}
              onChange={(people) =>
                setProcessors((p) => ({ ...p, absencesValidatorsProfEcole: people }))
              }
            />
          </div>
        </label>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Professeurs — collège (validation)
          </span>
          <div className="mt-2">
            <DirectoryPeoplePersonSelect
              members={members}
              selected={processors.absencesValidatorsProfCollege}
              onChange={(people) =>
                setProcessors((p) => ({ ...p, absencesValidatorsProfCollege: people }))
              }
            />
          </div>
        </label>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Professeurs — lycée (validation)
          </span>
          <div className="mt-2">
            <DirectoryPeoplePersonSelect
              members={members}
              selected={processors.absencesValidatorsProfLycee}
              onChange={(people) =>
                setProcessors((p) => ({ ...p, absencesValidatorsProfLycee: people }))
              }
            />
          </div>
        </label>
      </section>

      <section className="space-y-3">
        <div className="rounded-3xl border border-indigo-200 bg-indigo-50/50 p-5">
          <p className="text-[11px] font-black uppercase tracking-widest text-indigo-700">
            2 · Traitement
          </p>
          <h3 className="mt-1 font-black text-slate-900">Qui clôture après validation ?</h3>
          <p className="mt-1 text-sm text-slate-600">
            File « Dossiers à traiter » (pièces, déclaration rectorat / ONISE / RH). Professeurs :
            uniquement si déclaration instance (pas le rattrapage interne).
          </p>
        </div>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Professeurs — école (ONISE)
          </span>
          <div className="mt-2">
            <DirectoryPersonSelect
              members={members}
              selectedEmail={processors.absencesNotifyProfEcole?.email}
              selectedId={processors.absencesNotifyProfEcole?.userId}
              onChange={(m) => setTreatPerson("absencesNotifyProfEcole", m)}
            />
          </div>
        </label>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Professeurs — collège (rectorat)
          </span>
          <div className="mt-2">
            <DirectoryPersonSelect
              members={members}
              selectedEmail={processors.absencesNotifyProfCollege?.email}
              selectedId={processors.absencesNotifyProfCollege?.userId}
              onChange={(m) => setTreatPerson("absencesNotifyProfCollege", m)}
            />
          </div>
        </label>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Professeurs — lycée (rectorat)
          </span>
          <div className="mt-2">
            <DirectoryPersonSelect
              members={members}
              selectedEmail={processors.absencesNotifyProfLycee?.email}
              selectedId={processors.absencesNotifyProfLycee?.userId}
              onChange={(m) => setTreatPerson("absencesNotifyProfLycee", m)}
            />
          </div>
        </label>

        <label className="block rounded-3xl border border-slate-200 bg-white p-5">
          <span className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Personnel OGEC — RH / comptabilité
          </span>
          <div className="mt-2">
            <DirectoryPeopleSelect
              members={members}
              selectedEmails={processors.absencesNotifyOgecCompta}
              onChange={(emails) =>
                setProcessors((p) => ({ ...p, absencesNotifyOgecCompta: emails }))
              }
            />
          </div>
        </label>
      </section>

      {error ? <p className="text-sm font-medium text-rose-600">{error}</p> : null}
      {message ? <p className="text-sm font-medium text-emerald-700">{message}</p> : null}

      <button
        type="button"
        disabled={saving}
        onClick={() => void save()}
        className="rounded-2xl bg-indigo-600 px-5 py-3 text-sm font-bold text-white hover:bg-indigo-700 disabled:opacity-50"
      >
        {saving ? "Enregistrement…" : "Enregistrer"}
      </button>
    </div>
  );
}

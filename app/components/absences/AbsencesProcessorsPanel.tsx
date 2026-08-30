"use client";

import { useEffect, useState } from "react";
import DirectoryPersonSelect, {
  DirectoryPeopleSelect,
  directoryMemberLabel,
} from "@/app/components/settings/DirectoryPersonSelect";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import type { AbsenceNotifyPerson } from "@/app/lib/app-config-schemas";

type ProcessorsPayload = {
  absencesNotifyProfEcole: AbsenceNotifyPerson | null;
  absencesNotifyProfCollege: AbsenceNotifyPerson | null;
  absencesNotifyProfLycee: AbsenceNotifyPerson | null;
  absencesNotifyOgecCompta: string[];
};

export default function AbsencesProcessorsPanel() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [members, setMembers] = useState<DirectoryMemberOption[]>([]);
  const [processors, setProcessors] = useState<ProcessorsPayload>({
    absencesNotifyProfEcole: null,
    absencesNotifyProfCollege: null,
    absencesNotifyProfLycee: null,
    absencesNotifyOgecCompta: [],
  });

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/absences/processors", { cache: "no-store" });
        const data = (await res.json()) as {
          error?: string;
          processors?: ProcessorsPayload;
          members?: DirectoryMemberOption[];
          viewerCanConfigure?: boolean;
        };
        if (!res.ok) throw new Error(data.error || "Chargement impossible");
        if (!data.viewerCanConfigure) {
          throw new Error("Paramétrage réservé à la direction.");
        }
        if (!cancelled) {
          setProcessors({
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

  const setPerson = (key: keyof ProcessorsPayload, member: DirectoryMemberOption | null) => {
    if (key === "absencesNotifyOgecCompta") return;
    setProcessors((p) => ({
      ...p,
      [key]: member
        ? { label: directoryMemberLabel(member), email: member.email.trim(), userId: member.externalUserId }
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
      setMessage("Personnes enregistrées. Elles recevront un mail avec un lien pour traiter dans l’application.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <div className="rounded-3xl border border-slate-200 bg-white p-8 text-slate-500">Chargement…</div>;
  }

  return (
    <div className="space-y-4">
      <div className="rounded-3xl border border-slate-200 bg-white p-5">
        <h3 className="font-black text-slate-900">Qui traite après la direction ?</h3>
        <p className="mt-1 text-sm text-slate-600">
          Choisissez les personnes dans l’annuaire. Après validation direction, elles reçoivent un e-mail
          avec un lien vers l’intranet pour demander une pièce (sans repasser par la direction) puis
          clôturer le dossier.
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
            onChange={(m) => setPerson("absencesNotifyProfEcole", m)}
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
            onChange={(m) => setPerson("absencesNotifyProfCollege", m)}
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
            onChange={(m) => setPerson("absencesNotifyProfLycee", m)}
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
            onChange={(emails) => setProcessors((p) => ({ ...p, absencesNotifyOgecCompta: emails }))}
          />
        </div>
      </label>

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

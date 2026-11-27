"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "@/app/components/ui/AppLink";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SantaForm, { type SantaFormValues } from "@/app/components/secret-santa/SantaForm";

type Item = {
  id: string;
  title: string;
  organizerName: string;
  revealAt: string;
  participantCount: number;
  isOrganizer: boolean;
  awaitingMyAnswer: boolean;
};

const formatDate = (iso: string) =>
  new Date(iso).toLocaleString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  });

export default function SecretSantaListPage() {
  const router = useRouter();
  const [santas, setSantas] = useState<Item[] | null>(null);
  const [serverNow, setServerNow] = useState(0);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const res = await fetch("/api/secret-santa", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible.");
      setSantas(data.santas);
      setServerNow(new Date(data.serverNow).getTime());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible.");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function create(values: SantaFormValues) {
    const res = await fetch("/api/secret-santa", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(values),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Création impossible.");
    router.push(`/secret-santa/${data.id}`);
  }

  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Entre collègues"
        title="🎅 Secret Santa"
        description="Créez le vôtre, choisissez les participants et l'heure du tirage. Chacun ne découvre que le nom qui lui est destiné."
        actions={
          !creating ? (
            <button
              type="button"
              onClick={() => setCreating(true)}
              className="rounded-xl bg-red-600 px-4 py-2 text-sm font-bold text-white"
            >
              Organiser un Secret Santa
            </button>
          ) : null
        }
      />

      {error && (
        <p role="alert" className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      )}

      {creating ? (
        <SantaForm submitLabel="Créer le Secret Santa" onSubmit={create} onCancel={() => setCreating(false)} />
      ) : santas === null ? (
        !error && <p className="text-sm text-slate-400">Chargement…</p>
      ) : santas.length === 0 ? (
        <p className="rounded-2xl border border-dashed border-slate-300 p-8 text-center text-sm text-slate-500">
          Aucun Secret Santa pour l&apos;instant. Vous verrez ici ceux que vous organisez et ceux où l&apos;on vous
          invite.
        </p>
      ) : (
        <ul className="space-y-3">
          {santas.map((s) => (
            <li key={s.id}>
              <Link
                href={`/secret-santa/${s.id}`}
                className="block rounded-2xl border border-slate-200 bg-white p-4 hover:border-red-300"
              >
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-bold text-slate-900">{s.title}</span>
                  {s.awaitingMyAnswer && (
                    <span className="rounded-full bg-amber-100 px-2 py-0.5 text-xs font-bold text-amber-800">
                      À vous de répondre
                    </span>
                  )}
                  {s.isOrganizer && (
                    <span className="rounded-full bg-red-50 px-2 py-0.5 text-xs font-bold text-red-700">
                      J&apos;organise
                    </span>
                  )}
                </div>
                <p className="mt-1 text-sm text-slate-600">
                  {new Date(s.revealAt).getTime() > serverNow ? "Tirage" : "Tirage fait"} {formatDate(s.revealAt)} ·{" "}
                  {s.participantCount} participant(s) · organisé par {s.organizerName}
                </p>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </ModulePageShell>
  );
}

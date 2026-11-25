"use client";

/**
 * Contrôle des destinataires des invitations (circuit v2) : élève par élève, adresses prévues (masquées) avec leur
 * origine (fiche / foyer / ajoutée pour ce séjour), élèves sans adresse en rouge, et ajout d'une adresse POUR CE
 * SÉJOUR (jamais dans la fiche élève ni le foyer). Utilisé dans la validation finale et dans la vue Inscriptions.
 */

import { useCallback, useEffect, useState } from "react";
import { TripButton } from "@/app/components/travels/TripDetailUI";

type RecipientLine = { email: string; source: "fiche" | "foyer" | "ajoutee"; sourceLabel: string; name: string | null };
type StudentLine = { ine: string; name: string; classe: string | null; recipients: RecipientLine[] };
type Payload = { canWrite: boolean; students: StudentLine[]; withoutAddress: { ine: string; name: string }[] };

export function TripRecipientsCheck({
  tripId,
  onLoaded,
  onChanged,
  defaultOpen = false,
}: {
  tripId: string;
  /** Nombre d'élèves sans aucune adresse (à chaque chargement). */
  onLoaded?: (missing: number) => void;
  /** Appelé après l'ajout d'une adresse (la vue Inscriptions se recharge). */
  onChanged?: () => void;
  defaultOpen?: boolean;
}) {
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [open, setOpen] = useState(defaultOpen);
  const [adding, setAdding] = useState<string | null>(null);
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const res = await fetch(`/api/travels/v2/recipients?tripId=${encodeURIComponent(tripId)}`, { cache: "no-store" });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setData(j as Payload);
      setError(null);
      onLoaded?.((j as Payload).withoutAddress.length);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible");
    }
  }, [tripId, onLoaded]);

  useEffect(() => {
    void load();
  }, [load]);

  const submit = async (ine: string) => {
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/travels/v2/recipients", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, ine, email, name: name || undefined }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Ajout impossible");
      setMsg(
        j.opened
          ? j.sent
            ? "Adresse ajoutée : le lien vient d'être envoyé."
            : "Adresse ajoutée : l'invitation est créée, le lien partira à l'envoi suivant."
          : "Adresse ajoutée pour ce séjour : elle recevra le lien à l'ouverture.",
      );
      setAdding(null);
      setEmail("");
      setName("");
      await load();
      onChanged?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Ajout impossible");
    } finally {
      setBusy(false);
    }
  };

  if (!data) {
    return <p className="text-sm text-slate-500">{error ? `Destinataires : ${error}` : "Chargement des destinataires…"}</p>;
  }
  const missing = data.withoutAddress.length;
  const missingSet = new Set(data.withoutAddress.map((s) => s.ine));

  return (
    <div className="space-y-2 rounded-xl border bg-white p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm font-bold text-slate-800">
          Destinataires prévus ({data.students.length} élève{data.students.length > 1 ? "s" : ""})
        </p>
        <button type="button" className="text-xs font-semibold text-indigo-700 underline" onClick={() => setOpen((v) => !v)}>
          {open ? "Masquer le détail" : "Voir le détail"}
        </button>
      </div>
      {missing > 0 ? (
        <p className="rounded-lg border border-red-200 bg-red-50 p-2 text-sm font-semibold text-red-800" role="alert">
          {missing} élève{missing > 1 ? "s" : ""} sans aucune adresse : {data.withoutAddress.map((s) => s.name).join(", ")}.
          {data.canWrite ? " Ajoutez une adresse pour ce séjour ci-dessous." : ""}
        </p>
      ) : (
        <p className="text-xs font-semibold text-emerald-700">Chaque élève a au moins une adresse de responsable.</p>
      )}
      {error ? <p className="text-xs font-semibold text-red-700">{error}</p> : null}
      {msg ? <p className="text-xs font-semibold text-emerald-700">{msg}</p> : null}
      {open || missing > 0 ? (
        <ul className="max-h-72 divide-y overflow-y-auto text-sm">
          {data.students
            .filter((s) => open || missingSet.has(s.ine))
            .map((s) => (
              <li key={s.ine} className="py-1.5">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className={missingSet.has(s.ine) ? "font-bold text-red-700" : "font-semibold text-slate-800"}>
                    {s.name}
                    {s.classe ? <span className="ml-1 text-xs font-normal text-slate-500">({s.classe})</span> : null}
                  </span>
                  {data.canWrite ? (
                    <button type="button" className="text-xs font-semibold text-indigo-700 underline" onClick={() => setAdding(adding === s.ine ? null : s.ine)}>
                      Ajouter une adresse
                    </button>
                  ) : null}
                </div>
                {s.recipients.length === 0 ? (
                  <p className="text-xs font-semibold text-red-700">Aucune adresse : aucun lien ne partira.</p>
                ) : (
                  <ul className="text-xs text-slate-600">
                    {s.recipients.map((r) => (
                      <li key={r.email}>
                        {r.email}
                        {r.name ? ` — ${r.name}` : ""} <span className="text-slate-400">({r.sourceLabel})</span>
                      </li>
                    ))}
                  </ul>
                )}
                {adding === s.ine ? (
                  <div className="mt-2 flex flex-wrap items-end gap-2">
                    <label className="text-xs">
                      <span className="block font-semibold text-slate-600">Adresse e-mail</span>
                      <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} maxLength={254} className="rounded-lg border p-1.5 text-sm" />
                    </label>
                    <label className="text-xs">
                      <span className="block font-semibold text-slate-600">Nom du responsable (facultatif)</span>
                      <input type="text" value={name} onChange={(e) => setName(e.target.value)} maxLength={100} className="rounded-lg border p-1.5 text-sm" />
                    </label>
                    <TripButton size="sm" disabled={busy || !email.trim()} onClick={() => void submit(s.ine)}>
                      {busy ? "Ajout…" : "Ajouter"}
                    </TripButton>
                    <p className="w-full text-[11px] text-slate-500">Valable pour ce séjour seulement : la fiche élève et le foyer ne sont pas modifiés.</p>
                  </div>
                ) : null}
              </li>
            ))}
        </ul>
      ) : null}
    </div>
  );
}

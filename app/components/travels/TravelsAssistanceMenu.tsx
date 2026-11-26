"use client";

import { useCallback, useEffect, useState } from "react";
import { TripButton } from "@/app/components/travels/TripDetailUI";
import type { TravelsAssistanceCardApiStatus } from "@/app/lib/travels-assistance-card-shared";
import type { EleveContacts } from "@/app/lib/travels-parent-contacts";

function telHref(raw: string | null | undefined): string | null {
  const s = String(raw ?? "").replace(/[^0-9+]/g, "");
  return s.length >= 6 ? `tel:${s}` : null;
}

type Participant = { eleveId: string; nom: string; prenom: string; classe: string | null };

async function post<T>(url: string, body: unknown): Promise<{ ok: boolean; status: number; data: T | null }> {
  try {
    const res = await fetch(url, {
      method: "POST",
      cache: "no-store",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const data = res.headers.get("content-type")?.includes("json") ? ((await res.json()) as T) : null;
    return { ok: res.ok, status: res.status, data };
  } catch {
    return { ok: false, status: 0, data: null };
  }
}

/**
 * Bouton « Assistance » d'un voyage : choix entre la carte d'assistance de l'assurance
 * et les contacts des parents (réservé organisateur / accompagnateurs du voyage / admin général).
 */
export function TravelsAssistanceMenu({
  tripId,
  status,
}: {
  tripId: string;
  status: TravelsAssistanceCardApiStatus | null | undefined;
}) {
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<"menu" | "contacts">("menu");
  const [canContacts, setCanContacts] = useState(false);
  const [participants, setParticipants] = useState<Participant[]>([]);
  const [query, setQuery] = useState("");
  const [current, setCurrent] = useState<EleveContacts | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void post<{ allowed?: boolean }>("/api/travels/parent-contacts", { action: "access", tripId }).then((r) => {
      if (!cancelled) setCanContacts(Boolean(r.ok && r.data?.allowed));
    });
    return () => {
      cancelled = true;
    };
  }, [tripId]);

  const close = useCallback(() => {
    setOpen(false);
    setView("menu");
    setCurrent(null);
    setQuery("");
    setError(null);
  }, []);

  const hasCard = Boolean(status?.configured && status.downloadUrl);
  if (!hasCard && !canContacts) return null;

  const openContacts = async () => {
    setView("contacts");
    setError(null);
    setBusy(true);
    const r = await post<{ participants?: Participant[]; error?: string }>("/api/travels/parent-contacts", {
      action: "participants",
      tripId,
    });
    setBusy(false);
    if (!r.ok) return setError(r.data?.error || "Chargement impossible.");
    setParticipants(r.data?.participants ?? []);
  };

  const pick = async (eleveId: string) => {
    setError(null);
    setBusy(true);
    const r = await post<{ contacts?: EleveContacts; error?: string }>("/api/travels/parent-contacts", {
      action: "eleve",
      tripId,
      eleveId,
    });
    setBusy(false);
    if (!r.ok || !r.data?.contacts) return setError(r.data?.error || "Chargement impossible.");
    setCurrent(r.data.contacts);
  };

  const downloadPdf = async () => {
    setError(null);
    setBusy(true);
    try {
      const res = await fetch("/api/travels/parent-contacts/pdf", {
        method: "POST",
        cache: "no-store",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId }),
      });
      if (!res.ok) throw new Error();
      const url = URL.createObjectURL(await res.blob());
      const a = document.createElement("a");
      a.href = url;
      a.download = "contacts-voyage-confidentiel.pdf";
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 10_000);
    } catch {
      setError("Téléchargement impossible.");
    } finally {
      setBusy(false);
    }
  };

  const q = query.trim().toLowerCase();
  const shown = q
    ? participants.filter((p) => `${p.prenom} ${p.nom}`.toLowerCase().includes(q) || `${p.nom} ${p.prenom}`.toLowerCase().includes(q))
    : participants;

  return (
    <>
      <TripButton
        variant="danger"
        size="md"
        className="shadow-lg shadow-rose-900/30 ring-2 ring-white/20"
        onClick={() => setOpen(true)}
      >
        🆘 Assistance
      </TripButton>
      {open && (
        <div
          className="fixed inset-0 z-[100] flex items-end justify-center bg-black/60 p-0 sm:items-center sm:p-4"
          role="dialog"
          aria-modal="true"
          aria-label="Assistance"
          onClick={(e) => e.target === e.currentTarget && close()}
        >
          <div className="max-h-[92vh] w-full max-w-lg overflow-y-auto rounded-t-2xl bg-white p-4 text-slate-900 shadow-2xl sm:rounded-2xl">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="text-lg font-bold">🆘 Assistance</h2>
              <button type="button" onClick={close} className="rounded-lg px-3 py-1 text-sm font-semibold text-slate-600">
                Fermer
              </button>
            </div>

            {view === "menu" && (
              <div className="space-y-3">
                {hasCard && (
                  <button
                    type="button"
                    className="w-full rounded-xl bg-rose-600 px-4 py-4 text-left text-base font-bold text-white"
                    onClick={() => window.open(status!.downloadUrl!, "_blank", "noopener,noreferrer")}
                  >
                    Assistance de l’assurance
                    <span className="block text-xs font-normal opacity-90">Carte d’assistance de l’établissement (PDF)</span>
                  </button>
                )}
                {canContacts && (
                  <button
                    type="button"
                    className="w-full rounded-xl bg-slate-800 px-4 py-4 text-left text-base font-bold text-white"
                    onClick={() => void openContacts()}
                  >
                    Contacts des parents
                    <span className="block text-xs font-normal opacity-90">Réservé à l’organisateur et aux accompagnateurs</span>
                  </button>
                )}
              </div>
            )}

            {view === "contacts" && (
              <div className="space-y-3">
                <button type="button" className="text-sm font-semibold text-slate-600" onClick={() => (current ? setCurrent(null) : setView("menu"))}>
                  ← Retour
                </button>
                {error && <p className="rounded-lg bg-rose-50 p-2 text-sm text-rose-700">{error}</p>}
                {busy && <p className="text-sm text-slate-500">Chargement…</p>}

                {!current && (
                  <>
                    <input
                      type="search"
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="Rechercher un élève du voyage"
                      className="w-full rounded-xl border border-slate-300 px-3 py-3 text-base"
                      autoFocus
                    />
                    <ul className="divide-y divide-slate-100 rounded-xl border border-slate-200">
                      {shown.map((p) => (
                        <li key={p.eleveId}>
                          <button type="button" className="w-full px-3 py-3 text-left text-base" onClick={() => void pick(p.eleveId)}>
                            <span className="font-semibold">
                              {p.prenom} {p.nom.toUpperCase()}
                            </span>
                            {p.classe && <span className="ml-2 text-xs text-slate-500">{p.classe}</span>}
                          </button>
                        </li>
                      ))}
                      {!busy && shown.length === 0 && <li className="px-3 py-3 text-sm text-slate-500">Aucun élève.</li>}
                    </ul>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void downloadPdf()}
                      className="w-full rounded-xl border-2 border-slate-800 px-4 py-3 text-sm font-bold disabled:opacity-50"
                    >
                      Télécharger la fiche contacts (PDF)
                    </button>
                    <p className="text-xs text-slate-500">Document confidentiel à emporter hors réseau et à détruire au retour. Chaque consultation est journalisée.</p>
                  </>
                )}

                {current && (
                  <div className="space-y-3">
                    <h3 className="text-base font-bold">
                      {current.prenom} {current.nom.toUpperCase()}
                      {current.classe && <span className="ml-2 text-sm font-normal text-slate-500">{current.classe}</span>}
                    </h3>
                    {current.groups.length === 0 && <p className="text-sm text-slate-600">Aucun contact enregistré pour cet élève.</p>}
                    {current.groups.map((g) => (
                      <section key={g.key} className="rounded-xl border border-slate-200 p-3">
                        <h4 className="mb-2 text-sm font-bold text-slate-700">
                          {g.titre}
                          {g.source === "fiche" && <span className="ml-2 rounded bg-amber-100 px-1.5 py-0.5 text-[11px] font-semibold text-amber-800">fiche élève</span>}
                        </h4>
                        <ul className="space-y-3">
                          {g.responsables.map((r, i) => {
                            const tel = telHref(r.telephone);
                            return (
                              <li key={i}>
                                <p className="text-sm font-semibold">
                                  {r.prenom} {r.nom.toUpperCase()}
                                  {r.lien && <span className="ml-2 font-normal text-slate-600">({r.lien})</span>}
                                  {r.autoriteParentale && (
                                    <span className="ml-2 rounded bg-emerald-100 px-1.5 py-0.5 text-[11px] font-semibold text-emerald-800">autorité parentale</span>
                                  )}
                                </p>
                                {tel ? (
                                  <a href={tel} className="mt-1 block rounded-xl bg-emerald-600 px-4 py-3 text-center text-base font-bold text-white">
                                    📞 {r.telephone}
                                  </a>
                                ) : (
                                  <p className="mt-1 text-sm text-slate-500">Pas de téléphone</p>
                                )}
                                {r.email && (
                                  <a href={`mailto:${r.email}`} className="mt-1 block text-sm text-blue-700 underline">
                                    {r.email}
                                  </a>
                                )}
                              </li>
                            );
                          })}
                        </ul>
                      </section>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}

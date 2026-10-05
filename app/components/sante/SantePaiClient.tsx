"use client";

import { useCallback, useEffect, useState } from "react";
import {
  SANTE_PAI_STATUT_LABELS,
  type SantePaiDocumentLite,
  type SantePaiRow,
  type SantePaiStatut,
} from "@/app/lib/sante-pai-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function statutClass(s: SantePaiStatut): string {
  if (s === "valide") return "border-emerald-200 bg-emerald-50 text-emerald-900";
  if (s === "brouillon") return "border-amber-200 bg-amber-50 text-amber-950";
  if (s === "revoque") return "border-slate-200 bg-slate-100 text-slate-600";
  return "border-rose-200 bg-rose-50 text-rose-900";
}

export default function SantePaiClient() {
  const [pais, setPais] = useState<SantePaiRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [editing, setEditing] = useState<SantePaiRow | null>(null);
  const [documents, setDocuments] = useState<SantePaiDocumentLite[]>([]);
  const [protocole, setProtocole] = useState("");
  const [traitements, setTraitements] = useState("");
  const [notes, setNotes] = useState("");
  const [documentId, setDocumentId] = useState<string>("");
  const [dateDebut, setDateDebut] = useState("");
  const [dateFin, setDateFin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const loadList = useCallback(async (eleveId?: string) => {
    const params = new URLSearchParams();
    if (eleveId) params.set("eleveId", eleveId);
    const res = await fetch(`/api/sante/pai?${params}`, { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      pais?: SantePaiRow[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setPais(data.pais ?? []);
  }, []);

  const loadDocs = useCallback(async (eleveId: string) => {
    const res = await fetch(
      `/api/sante/pai?eleveId=${encodeURIComponent(eleveId)}&documents=1`,
      { credentials: "include" },
    );
    const data = (await res.json().catch(() => ({}))) as { documents?: SantePaiDocumentLite[] };
    if (res.ok) setDocuments(data.documents ?? []);
  }, []);

  useEffect(() => {
    void loadList();
  }, [loadList]);

  useEffect(() => {
    if (q.trim().length < 2 || selected) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/sante/pai?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q, selected]);

  function fillForm(pai: SantePaiRow | null) {
    setEditing(pai);
    setProtocole(pai?.protocole ?? "");
    setTraitements(pai?.traitementsAutorises ?? "");
    setNotes(pai?.notes ?? "");
    setDocumentId(pai?.documentId ?? "");
    setDateDebut(pai?.dateDebut ?? "");
    setDateFin(pai?.dateFin ?? "");
  }

  async function selectEleve(e: EleveLite) {
    setSelected(e);
    setQ("");
    setHits([]);
    setError(null);
    setOkMsg(null);
    fillForm(null);
    await Promise.all([loadList(e.id), loadDocs(e.id)]);
  }

  async function ensureDocument() {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/sante/pai", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "ensure_document", eleveId: selected.id }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        document?: SantePaiDocumentLite;
        error?: string;
      };
      if (!res.ok) {
        setError(data.error || "Création document impossible.");
        return;
      }
      setDocumentId(data.document!.id);
      await loadDocs(selected.id);
      setOkMsg(`Référence document « ${data.document!.title} » prête.`);
    } finally {
      setBusy(false);
    }
  }

  async function creerOuSauver() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      if (editing) {
        const res = await fetch("/api/sante/pai", {
          method: "PATCH",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            id: editing.id,
            action: "update",
            protocole,
            traitementsAutorises: traitements,
            notes,
            documentId: documentId || null,
            dateDebut: dateDebut || null,
            dateFin: dateFin || null,
          }),
        });
        const data = (await res.json().catch(() => ({}))) as {
          pai?: SantePaiRow;
          error?: string;
        };
        if (!res.ok) {
          setError(data.error || "Enregistrement impossible.");
          return;
        }
        fillForm(data.pai!);
        setOkMsg("PAI mis à jour.");
      } else {
        const res = await fetch("/api/sante/pai", {
          method: "POST",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            eleveId: selected.id,
            protocole,
            traitementsAutorises: traitements,
            notes,
            documentId: documentId || null,
            dateDebut: dateDebut || null,
            dateFin: dateFin || null,
            ensureDocument: !documentId,
          }),
        });
        const data = (await res.json().catch(() => ({}))) as {
          pai?: SantePaiRow;
          error?: string;
        };
        if (!res.ok) {
          setError(data.error || "Création impossible.");
          return;
        }
        fillForm(data.pai!);
        setOkMsg("Brouillon PAI créé.");
        await loadDocs(selected.id);
      }
      await loadList(selected.id);
    } finally {
      setBusy(false);
    }
  }

  async function valider() {
    if (!editing) return;
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const saveRes = await fetch("/api/sante/pai", {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id: editing.id,
          action: "update",
          protocole,
          traitementsAutorises: traitements,
          notes,
          documentId: documentId || null,
          dateDebut: dateDebut || null,
          dateFin: dateFin || null,
        }),
      });
      const saveData = (await saveRes.json().catch(() => ({}))) as { error?: string };
      if (!saveRes.ok) {
        setError(saveData.error || "Enregistrement avant validation impossible.");
        return;
      }

      const res = await fetch("/api/sante/pai", {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: editing.id, action: "valider" }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        pai?: SantePaiRow;
        error?: string;
      };
      if (!res.ok) {
        setError(data.error || "Validation impossible.");
        return;
      }
      fillForm(data.pai!);
      setOkMsg(
        `PAI validé pour ${data.pai!.elevePrenom} ${data.pai!.eleveNom}. Les autres PAI valides de l’élève sont révoqués.`,
      );
      await loadList(selected?.id);
    } finally {
      setBusy(false);
    }
  }

  async function revoquer(id: string) {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/pai", {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, action: "revoquer" }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Révocation impossible.");
        return;
      }
      setOkMsg("PAI révoqué (conservé).");
      if (editing?.id === id) fillForm(null);
      await loadList(selected?.id);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Élève</h2>
        <p className="mt-1 text-sm text-slate-600">
          Le PDF reste dans le dossier. Ici : protocole, traitements autorisés, validation
          infirmerie.
        </p>
        <label className="mt-4 block text-sm font-semibold text-slate-800">
          Rechercher
          <input
            value={selected ? `${selected.prenom} ${selected.nom}` : q}
            onChange={(e) => {
              setSelected(null);
              fillForm(null);
              setQ(e.target.value);
            }}
            placeholder="Nom ou prénom"
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
          />
        </label>
        {!selected && hits.length > 0 ? (
          <ul className="mt-2 overflow-hidden rounded-xl border border-slate-200">
            {hits.map((e) => (
              <li key={e.id}>
                <button
                  type="button"
                  className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50"
                  onClick={() => void selectEleve(e)}
                >
                  <span className="font-semibold">
                    {e.prenom} {e.nom}
                  </span>
                  <span className="text-xs text-slate-500">{e.classe || "—"}</span>
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}
      {okMsg ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
          {okMsg}
        </p>
      ) : null}

      {selected ? (
        <section className="space-y-3 rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            {editing ? "Éditer le PAI" : "Nouveau PAI (brouillon)"}
            {editing ? (
              <span
                className={`ml-2 rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase ${statutClass(editing.statut)}`}
              >
                {SANTE_PAI_STATUT_LABELS[editing.statut]}
              </span>
            ) : null}
          </h2>
          <label className="block text-sm font-semibold text-slate-800">
            Protocole
            <textarea
              value={protocole}
              onChange={(e) => setProtocole(e.target.value)}
              rows={3}
              disabled={editing?.statut === "revoque"}
              placeholder="Gestes d’urgence, contacts, consignes…"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal disabled:opacity-60"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Traitements autorisés
            <textarea
              value={traitements}
              onChange={(e) => setTraitements(e.target.value)}
              rows={2}
              disabled={editing?.statut === "revoque"}
              placeholder="Ex. Ventoline — 2 bouffées si crise"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal disabled:opacity-60"
            />
          </label>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block text-sm font-semibold text-slate-800">
              Début
              <input
                type="date"
                value={dateDebut}
                onChange={(e) => setDateDebut(e.target.value)}
                disabled={editing?.statut === "revoque"}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              />
            </label>
            <label className="block text-sm font-semibold text-slate-800">
              Fin
              <input
                type="date"
                value={dateFin}
                onChange={(e) => setDateFin(e.target.value)}
                disabled={editing?.statut === "revoque"}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              />
            </label>
          </div>
          <label className="block text-sm font-semibold text-slate-800">
            Document lié
            <select
              value={documentId}
              onChange={(e) => setDocumentId(e.target.value)}
              disabled={editing?.statut === "revoque"}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            >
              <option value="">— aucun / créer à l’enregistrement —</option>
              {documents.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.title}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            disabled={busy || editing?.statut === "revoque"}
            onClick={() => void ensureDocument()}
            className="rounded-xl border border-slate-300 px-3 py-1.5 text-xs font-bold text-slate-800 hover:bg-slate-50 disabled:opacity-50"
          >
            Créer une référence document PAI
          </button>
          <label className="block text-sm font-semibold text-slate-800">
            Notes
            <input
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              disabled={editing?.statut === "revoque"}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={busy || editing?.statut === "revoque"}
              onClick={() => void creerOuSauver()}
              className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              {editing ? "Enregistrer" : "Créer le brouillon"}
            </button>
            {editing && editing.statut !== "revoque" && editing.statut !== "valide" ? (
              <button
                type="button"
                disabled={busy || !protocole.trim() || !traitements.trim()}
                onClick={() => void valider()}
                className="rounded-xl bg-emerald-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
              >
                Valider (infirmerie)
              </button>
            ) : null}
            {editing ? (
              <button
                type="button"
                disabled={busy}
                onClick={() => fillForm(null)}
                className="rounded-xl border border-slate-300 px-3 py-2 text-xs font-bold text-slate-700"
              >
                Nouveau brouillon
              </button>
            ) : null}
          </div>
        </section>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          PAI
          {selected ? (
            <span className="ml-2 text-xs font-medium normal-case text-slate-500">
              — {selected.prenom} {selected.nom}
            </span>
          ) : (
            <span className="ml-2 text-xs font-medium normal-case text-slate-500">— récents</span>
          )}
        </h2>
        {pais.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucun PAI enregistré.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {pais.map((p) => (
              <li
                key={p.id}
                className="flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {p.elevePrenom} {p.eleveNom}
                    <span
                      className={`ml-2 rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase ${statutClass(p.statut)}`}
                    >
                      {SANTE_PAI_STATUT_LABELS[p.statut]}
                    </span>
                  </p>
                  <p className="text-sm text-slate-700 line-clamp-2">{p.protocole || "—"}</p>
                  <p className="text-xs text-slate-500">
                    Traitements : {p.traitementsAutorises || "—"}
                    {p.documentTitle ? ` · Doc : ${p.documentTitle}` : ""}
                    {p.valideParNom ? ` · Validé par ${p.valideParNom}` : ""}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => {
                      void (async () => {
                        if (!selected || selected.id !== p.eleveId) {
                          setSelected({
                            id: p.eleveId,
                            nom: p.eleveNom,
                            prenom: p.elevePrenom,
                            classe: p.eleveClasse,
                          });
                          setQ("");
                          setHits([]);
                          await Promise.all([loadList(p.eleveId), loadDocs(p.eleveId)]);
                        }
                        fillForm(p);
                      })();
                    }}
                    className="rounded-xl border border-slate-300 px-3 py-1.5 text-xs font-bold text-slate-800"
                  >
                    Ouvrir
                  </button>
                  {p.statut !== "revoque" ? (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void revoquer(p.id)}
                      className="rounded-xl border border-slate-300 px-3 py-1.5 text-xs font-bold text-slate-600"
                    >
                      Révoquer
                    </button>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

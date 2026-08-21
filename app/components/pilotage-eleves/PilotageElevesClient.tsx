"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import PilotageDirectionNotes from "@/app/components/pilotage-eleves/PilotageDirectionNotes";
import { dash } from "@/app/lib/dashboard-brand";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { useOneDriveConnection } from "@/app/hooks/useOneDriveConnection";
import {
  canonicalClasseLabel,
  PIECE_KIND_LABEL,
} from "@/app/lib/pilotage-eleves-logic";
import type {
  PilotageEleveDossier,
  PilotageEleveSummary,
  PilotageFlashPoint,
  PilotageOverview,
  PilotagePiece,
} from "@/app/lib/pilotage-eleves-types";

const SECTEUR_LABEL: Record<Secteur, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
};

type FichePayload = {
  eleve: {
    key: string;
    nom: string;
    prenom: string;
    classe: string;
    folderName: string;
    secteur: Secteur;
  };
  dossier: PilotageEleveDossier | null;
};

function encodeDrivePath(path: string): string {
  return path
    .split("/")
    .filter(Boolean)
    .map((s) => encodeURIComponent(s))
    .join("/");
}

async function openPiece(piece: PilotagePiece, token: string | null) {
  const direct = piece.shareUrl || piece.webUrl;
  if (direct) {
    window.open(direct, "_blank", "noopener,noreferrer");
    return;
  }
  if (!token || !piece.path) {
    throw new Error("Document pas encore indexé — synchronisez le dossier.");
  }
  const res = await fetch(
    `https://graph.microsoft.com/v1.0/me/drive/root:/${encodeDrivePath(piece.path)}:?$select=webUrl`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  if (!res.ok) throw new Error("Ouverture OneDrive impossible.");
  const data = (await res.json()) as { webUrl?: string };
  if (!data.webUrl) throw new Error("Lien OneDrive introuvable.");
  window.open(data.webUrl, "_blank", "noopener,noreferrer");
}

export default function PilotageElevesClient() {
  const od = useOneDriveConnection();
  const [overview, setOverview] = useState<PilotageOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [classe, setClasse] = useState<{ secteur: Secteur; classe: string } | null>(null);
  const [roster, setRoster] = useState<PilotageEleveSummary[]>([]);
  const [index, setIndex] = useState(0);
  const [fiche, setFiche] = useState<FichePayload | null>(null);
  const [loadingFiche, setLoadingFiche] = useState(false);
  const [rosterQuery, setRosterQuery] = useState("");
  const [indexing, setIndexing] = useState(false);
  const [syncLabel, setSyncLabel] = useState<string | null>(null);
  const touchX = useRef<number | null>(null);
  const indexRef = useRef(0);
  const rosterRef = useRef<PilotageEleveSummary[]>([]);
  indexRef.current = index;
  rosterRef.current = roster;

  const loadOverview = useCallback(async () => {
    const res = await fetch("/api/pilotage-eleves/overview", { cache: "no-store" });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Accès refusé.");
      return;
    }
    setOverview(data);
    setError(null);
  }, []);

  useEffect(() => {
    void loadOverview();
  }, [loadOverview]);

  const loadFiche = useCallback(async (key: string) => {
    setLoadingFiche(true);
    try {
      const res = await fetch(`/api/pilotage-eleves/eleve/${encodeURIComponent(key)}`, {
        cache: "no-store",
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Fiche introuvable.");
        return;
      }
      setFiche(data);
      setError(null);
    } finally {
      setLoadingFiche(false);
    }
  }, []);

  const openClass = async (secteur: Secteur, classeName: string) => {
    setClasse({ secteur, classe: classeName });
    setFiche(null);
    setRosterQuery("");
    const res = await fetch(
      `/api/pilotage-eleves/classe?secteur=${encodeURIComponent(secteur)}&classe=${encodeURIComponent(classeName)}`,
      { cache: "no-store" },
    );
    const data = await res.json();
    const eleves = (data.eleves ?? []) as PilotageEleveSummary[];
    setRoster(eleves);
    setIndex(-1);
  };

  const openEleve = (i: number) => {
    const row = roster[i];
    if (!row) return;
    setIndex(i);
    void loadFiche(row.key);
  };

  const go = useCallback(
    (delta: number) => {
      const list = rosterRef.current;
      if (!list.length || indexRef.current < 0) return;
      const next = Math.max(0, Math.min(list.length - 1, indexRef.current + delta));
      if (next === indexRef.current) return;
      setIndex(next);
      const row = list[next];
      if (row) void loadFiche(row.key);
    },
    [loadFiche],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!classe || indexRef.current < 0) return;
      if (e.key === "ArrowRight") go(1);
      if (e.key === "ArrowLeft") go(-1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [classe, go]);

  const current = roster[index];

  const indexCurrent = async () => {
    if (!current || !overview?.canIndex) return;
    const token = await od.ensureToken();
    if (!token) {
      await od.login();
      return;
    }
    setIndexing(true);
    try {
      const res = await fetch("/api/pilotage-eleves/index", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessToken: token, key: current.key }),
      });
      const data = await res.json();
      if (!res.ok) setError(data.error || "Indexation impossible.");
      else await loadFiche(current.key);
    } finally {
      setIndexing(false);
    }
  };

  const syncSecteur = async (secteur: Secteur) => {
    if (!overview?.canIndex) return;
    const token = await od.ensureToken();
    if (!token) {
      await od.login();
      return;
    }
    setIndexing(true);
    setError(null);
    try {
      let offset = 0;
      let done = false;
      let total = 0;
      while (!done) {
        const res = await fetch("/api/pilotage-eleves/index", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ accessToken: token, secteur, syncAll: true, offset }),
        });
        const data = await res.json();
        if (!res.ok) {
          setError(data.error || "Synchronisation impossible.");
          break;
        }
        total = Number(data.total) || total;
        offset = Number(data.nextOffset) || offset;
        done = Boolean(data.done);
        setSyncLabel(
          total ? `Synchronisation ${Math.min(offset, total)} / ${total} dossiers…` : "Synchronisation…",
        );
      }
      await loadOverview();
      if (classe?.secteur === secteur) await openClass(classe.secteur, classe.classe);
    } finally {
      setIndexing(false);
      setSyncLabel(null);
    }
  };

  const grouped = useMemo(() => {
    if (!overview) return [];
    const q = query.trim().toLowerCase();
    const map = new Map<Secteur, typeof overview.classes>();
    for (const c of overview.classes) {
      if (q && !c.classe.toLowerCase().includes(q)) continue;
      const list = map.get(c.secteur) ?? [];
      list.push(c);
      map.set(c.secteur, list);
    }
    return [...map.entries()];
  }, [overview, query]);

  if (!overview && !error) {
    return (
      <ModulePageShell>
        <p className={`text-center text-sm ${dash.textMid}`}>Chargement…</p>
      </ModulePageShell>
    );
  }

  if (error && !overview) {
    return (
      <ModulePageShell>
        <p className="text-center text-sm text-red-600">{error}</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1180px]">
      <ModulePageHeader
        eyebrow="Confidentiel"
        title="Pilotage documentaire"
        description="Conseil de classe à partir des dossiers classés (bulletins, PAP/PAI…). Pas de vie scolaire live. Visible uniquement secrétariat OCR et direction du même cycle."
        actions={
          overview?.canIndex && overview.secteurs[0] ? (
            <ModuleButton disabled={indexing} onClick={() => void syncSecteur(overview.secteurs[0]!)}>
              {indexing && syncLabel ? syncLabel : "Synchroniser tout le cycle"}
            </ModuleButton>
          ) : undefined
        }
      />

      {error ? <p className="mb-3 text-sm text-red-600">{error}</p> : null}

      {!classe ? (
        <div className="space-y-6">
          {overview && overview.secteurs.length === 0 ? (
            <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
              Votre compte n’est rattaché ni à un flux OCR élèves, ni à une direction d’établissement. Le
              module reste vide — c’est volontaire.
            </div>
          ) : null}

          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Rechercher une classe…"
            className="w-full max-w-md rounded-2xl border border-white/80 bg-white/90 px-4 py-2.5 text-sm shadow-sm outline-none ring-[var(--dash-primary)]/20 focus:ring-2"
          />

          {grouped.map(([secteur, classes]) => (
            <section key={secteur}>
              <div className="mb-3 flex flex-wrap items-end justify-between gap-2">
                <div>
                  <h2 className={`text-lg font-semibold ${dash.ink}`}>{SECTEUR_LABEL[secteur]}</h2>
                  <p className={`text-xs ${dash.textMid}`}>
                    {classes.reduce((n, c) => n + c.count, 0)} élèves · {classes.length} classe
                    {classes.length > 1 ? "s" : ""}
                  </p>
                </div>
                {overview?.canIndex ? (
                  <button
                    type="button"
                    disabled={indexing}
                    onClick={() => void syncSecteur(secteur)}
                    className={`text-xs font-semibold ${dash.textPrimary} disabled:opacity-50`}
                  >
                    Synchroniser {SECTEUR_LABEL[secteur]}
                  </button>
                ) : null}
              </div>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {classes.map((c) => (
                  <button
                    key={`${c.secteur}-${c.classe}`}
                    type="button"
                    onClick={() => void openClass(c.secteur, c.classe)}
                    className="group rounded-3xl border border-white/80 bg-white/90 p-4 text-left shadow-sm transition hover:-translate-y-0.5 hover:shadow-md"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <p className={`text-lg font-semibold tracking-tight ${dash.ink}`}>{c.classe}</p>
                      <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-semibold text-slate-600">
                        {c.count}
                      </span>
                    </div>
                    <p className={`mt-2 text-xs ${dash.textMid}`}>
                      {c.missingBulletin > 0
                        ? `${c.missingBulletin} sans bulletin indexé`
                        : "Bulletins présents"}
                      {c.drops > 0 ? ` · ${c.drops} chute${c.drops > 1 ? "s" : ""}` : ""}
                    </p>
                    {c.drops > 0 ? (
                      <span className="mt-3 inline-flex rounded-full bg-red-50 px-2 py-0.5 text-[11px] font-semibold text-red-700">
                        À regarder
                      </span>
                    ) : null}
                  </button>
                ))}
              </div>
            </section>
          ))}
          {overview?.classes.length === 0 && overview.secteurs.length > 0 ? (
            <p className={`text-sm ${dash.textMid}`}>Aucun élève dans votre périmètre.</p>
          ) : null}
        </div>
      ) : (
        <div
          className="space-y-4"
          onTouchStart={(e) => {
            if (!fiche) return;
            touchX.current = e.changedTouches[0]?.clientX ?? null;
          }}
          onTouchEnd={(e) => {
            if (!fiche) return;
            const start = touchX.current;
            const end = e.changedTouches[0]?.clientX;
            if (start == null || end == null) return;
            const d = end - start;
            if (d > 70) go(-1);
            if (d < -70) go(1);
          }}
        >
          <div className="sticky top-2 z-10 flex flex-wrap items-center gap-2 rounded-2xl border border-white/80 bg-white/90 px-3 py-2 shadow-sm backdrop-blur">
            <ModuleButton
              variant="secondary"
              onClick={() => {
                if (fiche) {
                  setFiche(null);
                  setIndex(-1);
                  return;
                }
                setClasse(null);
              }}
            >
              {fiche ? "← Liste" : "← Classes"}
            </ModuleButton>
            <p className={`text-sm font-semibold ${dash.ink}`}>
              {SECTEUR_LABEL[classe.secteur]} · {classe.classe}
            </p>
            {fiche && current ? (
              <p className={`text-sm ${dash.textMid}`}>
                {index + 1} / {roster.length} — {current.nom} {current.prenom}
              </p>
            ) : (
              <p className={`text-sm ${dash.textMid}`}>{roster.length} élèves</p>
            )}
            {fiche ? (
              <div className="ml-auto flex gap-2">
                <ModuleButton variant="secondary" onClick={() => go(-1)} disabled={index <= 0}>
                  Précédent
                </ModuleButton>
                <ModuleButton variant="secondary" onClick={() => go(1)} disabled={index >= roster.length - 1}>
                  Suivant
                </ModuleButton>
              </div>
            ) : null}
          </div>

          {!fiche ? (
            <>
              {loadingFiche ? (
                <p className={`text-sm ${dash.textMid}`}>Ouverture de la fiche…</p>
              ) : null}
              <RosterView
                roster={roster}
                query={rosterQuery}
                onQuery={setRosterQuery}
                onOpen={openEleve}
              />
            </>
          ) : (
            <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)]">
              <aside className="hidden max-h-[70vh] overflow-y-auto rounded-2xl border border-white/80 bg-white/90 p-2 lg:block">
                {roster.map((e, i) => (
                  <button
                    key={e.key}
                    type="button"
                    onClick={() => openEleve(i)}
                    className={`mb-0.5 w-full rounded-xl px-2.5 py-1.5 text-left text-sm ${
                      i === index ? "bg-slate-900 text-white" : "hover:bg-slate-50"
                    }`}
                  >
                    <span className="block truncate font-medium">
                      {e.nom} {e.prenom}
                    </span>
                  </button>
                ))}
              </aside>
              <div className="min-w-0">
                <div className="mb-3 flex gap-1 overflow-x-auto lg:hidden">
                  {roster.map((e, i) => (
                    <button
                      key={e.key}
                      type="button"
                      onClick={() => openEleve(i)}
                      className={`shrink-0 rounded-full px-3 py-1 text-xs font-semibold ${
                        i === index ? "bg-slate-900 text-white" : "bg-white text-slate-700"
                      }`}
                    >
                      {e.prenom}
                    </button>
                  ))}
                </div>
                <FicheView
                  fiche={fiche}
                  canIndex={Boolean(overview?.canIndex)}
                  canWriteNotes={Boolean(overview?.canWriteNotes)}
                  indexing={indexing}
                  onIndex={indexCurrent}
                  od={od}
                />
              </div>
            </div>
          )}
        </div>
      )}
    </ModulePageShell>
  );
}

function RosterView({
  roster,
  query,
  onQuery,
  onOpen,
}: {
  roster: PilotageEleveSummary[];
  query: string;
  onQuery: (v: string) => void;
  onOpen: (i: number) => void;
}) {
  const q = query.trim().toLowerCase();
  const rows = roster
    .map((e, i) => ({ e, i }))
    .filter(({ e }) => !q || `${e.nom} ${e.prenom}`.toLowerCase().includes(q));

  return (
    <div className="space-y-3">
      <input
        value={query}
        onChange={(e) => onQuery(e.target.value)}
        placeholder="Chercher un élève…"
        className="w-full max-w-md rounded-2xl border border-white/80 bg-white/90 px-4 py-2.5 text-sm shadow-sm outline-none"
      />
      <ul className="overflow-hidden rounded-3xl border border-white/80 bg-white/90 shadow-sm">
        {rows.map(({ e, i }) => (
          <li key={e.key} className="border-b border-slate-100 last:border-0">
            <button
              type="button"
              onClick={() => onOpen(i)}
              className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left hover:bg-slate-50"
            >
              <span>
                <span className={`block font-semibold ${dash.ink}`}>
                  {e.nom} {e.prenom}
                </span>
                <span className={`text-xs ${dash.textMid}`}>
                  {e.emptyDossier ? "Dossier non indexé" : e.hasBulletin ? "Bulletins" : "Pas de bulletin"}
                  {e.hasPapPaiPps ? " · PAP/PAI" : ""}
                  {typeof e.lastMoyenne === "number" ? ` · ${e.lastMoyenne.toFixed(1)}` : ""}
                </span>
              </span>
              {e.dropSignal ? (
                <span className="rounded-full bg-red-50 px-2 py-0.5 text-[11px] font-semibold text-red-700">
                  Chute
                </span>
              ) : null}
            </button>
          </li>
        ))}
      </ul>
      {rows.length === 0 ? <p className={`text-sm ${dash.textMid}`}>Aucun élève.</p> : null}
    </div>
  );
}

function flashClass(tone: PilotageFlashPoint["tone"]): string {
  if (tone === "alert") return "border-red-200 bg-red-50 text-red-950";
  if (tone === "watch") return "border-amber-200 bg-amber-50 text-amber-950";
  if (tone === "plus") return "border-emerald-200 bg-emerald-50 text-emerald-950";
  return "border-slate-200 bg-slate-50 text-slate-800";
}

function FicheView({
  fiche,
  canIndex,
  canWriteNotes,
  indexing,
  onIndex,
  od,
}: {
  fiche: FichePayload;
  canIndex: boolean;
  canWriteNotes: boolean;
  indexing: boolean;
  onIndex: () => void;
  od: ReturnType<typeof useOneDriveConnection>;
}) {
  const d = fiche.dossier;
  const bulletins = d?.bulletins ?? [];
  const niveaux = d?.moyennesParNiveau ?? [];
  const points = d?.synthese?.points ?? [];
  const [openErr, setOpenErr] = useState<string | null>(null);

  const onOpen = async (piece: PilotagePiece) => {
    setOpenErr(null);
    try {
      const token = canIndex ? await od.ensureToken() : od.accessToken;
      await openPiece(piece, token);
    } catch (e) {
      setOpenErr(e instanceof Error ? e.message : "Ouverture impossible");
    }
  };

  return (
    <div className="space-y-4">
      <section className="rounded-3xl border border-white/80 bg-white/90 p-5 shadow-sm">
        <p className={`text-xs font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
          {canonicalClasseLabel(fiche.eleve.classe)}
        </p>
        <h2 className={`mt-1 text-2xl font-semibold tracking-tight ${dash.ink}`}>
          {fiche.eleve.nom} {fiche.eleve.prenom}
        </h2>
        <p className={`mt-1 text-sm ${dash.textMid}`}>
          Documents officiels du dossier élève — pas Charlemagne / École Directe
        </p>
        {d?.drop ? (
          <p
            className={`mt-3 rounded-2xl px-3 py-2 text-sm ${
              d.drop.kind === "drop" ? "bg-red-50 text-red-800" : "bg-slate-50 text-slate-700"
            }`}
          >
            {d.drop.detail}
          </p>
        ) : null}
        <div className="mt-3 flex flex-wrap gap-1.5">
          {d?.flags.hasPap ? <Flag>PAP</Flag> : null}
          {d?.flags.hasPai ? <Flag>PAI</Flag> : null}
          {d?.flags.hasPps ? <Flag>PPS</Flag> : null}
          {!d || d.pieces.length === 0 ? <Flag muted>Dossier non indexé</Flag> : null}
        </div>
        {canIndex ? (
          <div className="mt-4">
            <ModuleButton onClick={onIndex} disabled={indexing}>
              {indexing ? "Indexation…" : "Actualiser ce dossier"}
            </ModuleButton>
            {od.error ? <p className="mt-1 text-xs text-red-600">{od.error}</p> : null}
          </div>
        ) : null}
      </section>

      <section className="rounded-3xl border border-white/80 bg-white/90 p-5 shadow-sm">
        <h3 className={`mb-3 font-semibold ${dash.ink}`}>En un coup d’œil</h3>
        {points.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>
            Pas encore de points flash — synchronisez le dossier pour croiser les bulletins.
          </p>
        ) : (
          <ul className="grid gap-2 sm:grid-cols-2">
            {points.map((p, i) => (
              <li key={`${p.titre}-${i}`} className={`rounded-2xl border px-3 py-2.5 ${flashClass(p.tone)}`}>
                <p className="text-[11px] font-semibold uppercase tracking-wide opacity-70">
                  {p.tone === "alert"
                    ? "Attention"
                    : p.tone === "watch"
                      ? "Vigilance"
                      : p.tone === "plus"
                        ? "Point fort"
                        : "Constat"}
                </p>
                <p className="mt-0.5 font-semibold leading-snug">{p.titre}</p>
                {p.detail ? <p className="mt-1 text-xs leading-relaxed opacity-80">{p.detail}</p> : null}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-3xl border border-white/80 bg-white/90 p-5 shadow-sm">
        <h3 className={`mb-3 font-semibold ${dash.ink}`}>Moyennes d’année</h3>
        {niveaux.length === 0 && bulletins.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>
            Aucun bulletin extrait. Le secrétariat peut synchroniser le dossier.
          </p>
        ) : (
          <>
            {niveaux.length > 0 ? (
              <div className="mb-3 flex gap-2 overflow-x-auto pb-1">
                {niveaux.map((n) => (
                  <div key={n.niveau} className="min-w-[120px] shrink-0 rounded-2xl bg-slate-900 px-3 py-2 text-white">
                    <p className="text-[11px] font-semibold uppercase tracking-wide text-white/70">{n.niveau}</p>
                    <p className="text-xl font-semibold">{n.moyenne.toFixed(1)}</p>
                    <p className="text-[11px] text-white/60">
                      {n.periodes.length > 1 ? n.periodes.join(" + ") : n.periodes[0] || n.anneeScolaire}
                    </p>
                  </div>
                ))}
              </div>
            ) : null}
            <p className={`mb-2 text-xs ${dash.textMid}`}>Périodes (cliquer pour ouvrir le PDF)</p>
            <div className="flex gap-2 overflow-x-auto pb-2">
              {bulletins.map((b) => {
                const piece = d?.pieces.find((p) => p.id === b.pieceId);
                return (
                  <button
                    key={b.pieceId}
                    type="button"
                    disabled={!piece}
                    onClick={() => piece && void onOpen(piece)}
                    className="min-w-[140px] shrink-0 rounded-2xl bg-slate-50 px-3 py-2 text-left text-sm transition hover:bg-slate-100 disabled:opacity-60"
                  >
                    <p className="text-[11px] font-semibold uppercase tracking-wide text-slate-500">
                      {[canonicalClasseLabel(b.classe), b.periode].filter(Boolean).join(" · ") || "Bulletin"}
                    </p>
                    <p className={`mt-1 text-lg font-semibold ${dash.ink}`}>
                      {typeof b.moyenneGenerale === "number" ? b.moyenneGenerale.toFixed(1) : "—"}
                    </p>
                    <p className="text-[11px] text-slate-500">{b.anneeScolaire || "année ?"}</p>
                  </button>
                );
              })}
            </div>
          </>
        )}
      </section>

      <section className="rounded-3xl border border-white/80 bg-white/90 p-5 shadow-sm">
        <h3 className={`mb-3 font-semibold ${dash.ink}`}>Pièces du dossier</h3>
        {openErr ? <p className="mb-2 text-xs text-red-600">{openErr}</p> : null}
        {!d || d.pieces.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>Pas encore de pièces indexées.</p>
        ) : (
          <ul className="grid gap-2 sm:grid-cols-2">
            {d.pieces.map((p) => (
              <li key={p.id}>
                <button
                  type="button"
                  onClick={() => void onOpen(p)}
                  className="flex w-full items-center justify-between gap-2 rounded-2xl border border-slate-100 bg-slate-50 px-3 py-2.5 text-left text-sm transition hover:border-[color:var(--dash-primary)]/30 hover:bg-white"
                >
                  <span className="min-w-0 truncate font-medium">{p.name.replace(/\.pdf$/i, "")}</span>
                  <span className="shrink-0 rounded-full bg-white px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-slate-600">
                    {PIECE_KIND_LABEL[p.kind]}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      {d?.synthese?.sources?.length ? (
        <p className={`text-xs ${dash.textMid}`}>Sources : {d.synthese.sources.join(" · ")}</p>
      ) : null}

      {canWriteNotes ? (
        <PilotageDirectionNotes
          secteur={fiche.eleve.secteur}
          classe={fiche.eleve.classe}
          folderName={fiche.eleve.folderName}
          eleveLabel={`${fiche.eleve.nom} ${fiche.eleve.prenom}`}
          od={od}
        />
      ) : canIndex ? (
        <p className={`text-xs ${dash.textMid}`}>
          Les notes de classeur de la direction ne sont pas visibles du secrétariat.
        </p>
      ) : null}
    </div>
  );
}

function Flag({ children, muted }: { children: string; muted?: boolean }) {
  return (
    <span
      className={`rounded-full px-2 py-0.5 text-[11px] font-semibold ${
        muted ? "bg-slate-100 text-slate-600" : "bg-violet-50 text-violet-800"
      }`}
    >
      {children}
    </span>
  );
}

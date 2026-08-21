"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import PilotageDirectionNotes from "@/app/components/pilotage-eleves/PilotageDirectionNotes";
import { dash } from "@/app/lib/dashboard-brand";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { useOneDriveConnection } from "@/app/hooks/useOneDriveConnection";
import type {
  PilotageEleveDossier,
  PilotageEleveSummary,
  PilotageOverview,
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

export default function PilotageElevesClient() {
  const od = useOneDriveConnection();
  const [overview, setOverview] = useState<PilotageOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [classe, setClasse] = useState<{ secteur: Secteur; classe: string } | null>(null);
  const [roster, setRoster] = useState<PilotageEleveSummary[]>([]);
  const [index, setIndex] = useState(0);
  const [fiche, setFiche] = useState<FichePayload | null>(null);
  const [loadingFiche, setLoadingFiche] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const touchX = useRef<number | null>(null);

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

  const openClass = async (secteur: Secteur, classeName: string) => {
    setClasse({ secteur, classe: classeName });
    setFiche(null);
    const res = await fetch(
      `/api/pilotage-eleves/classe?secteur=${encodeURIComponent(secteur)}&classe=${encodeURIComponent(classeName)}`,
      { cache: "no-store" },
    );
    const data = await res.json();
    const eleves = (data.eleves ?? []) as PilotageEleveSummary[];
    setRoster(eleves);
    setIndex(0);
    if (eleves[0]) void loadFiche(eleves[0].key);
  };

  const loadFiche = async (key: string) => {
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
  };

  const go = (delta: number) => {
    if (!roster.length) return;
    const next = Math.max(0, Math.min(roster.length - 1, index + delta));
    if (next === index) return;
    setIndex(next);
    const row = roster[next];
    if (row) void loadFiche(row.key);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!classe) return;
      if (e.key === "ArrowRight") go(1);
      if (e.key === "ArrowLeft") go(-1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

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

  const grouped = useMemo(() => {
    if (!overview) return [];
    const map = new Map<Secteur, typeof overview.classes>();
    for (const c of overview.classes) {
      const list = map.get(c.secteur) ?? [];
      list.push(c);
      map.set(c.secteur, list);
    }
    return [...map.entries()];
  }, [overview]);

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
    <ModulePageShell maxWidthClass="max-w-[1100px]">
      <ModulePageHeader
        eyebrow="Élèves"
        title="Pilotage documentaire"
        description="Aide au conseil de classe à partir des dossiers officiels classés (bulletins, PAP/PAI…). Ce n’est pas Charlemagne ni École Directe : pas d’absences live, pas de sanctions."
      />

      {error ? <p className="mb-3 text-sm text-red-600">{error}</p> : null}

      {!classe ? (
        <div className="space-y-6">
          {grouped.map(([secteur, classes]) => (
            <section key={secteur}>
              <h2 className={`mb-2 text-lg font-semibold ${dash.ink}`}>{SECTEUR_LABEL[secteur]}</h2>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                {classes.map((c) => (
                  <button
                    key={`${c.secteur}-${c.classe}`}
                    type="button"
                    onClick={() => void openClass(c.secteur, c.classe)}
                    className="rounded-2xl border border-white/70 bg-white/80 px-4 py-3 text-left shadow-sm transition hover:-translate-y-0.5"
                  >
                    <p className={`font-semibold ${dash.ink}`}>{c.classe}</p>
                    <p className={`mt-1 text-xs ${dash.textMid}`}>
                      {c.count} élève{c.count > 1 ? "s" : ""}
                      {c.alerts > 0 ? ` · ${c.alerts} signal${c.alerts > 1 ? "s" : ""}` : ""}
                    </p>
                  </button>
                ))}
              </div>
            </section>
          ))}
          {overview?.classes.length === 0 ? (
            <p className={`text-sm ${dash.textMid}`}>Aucun élève dans votre périmètre.</p>
          ) : null}
        </div>
      ) : (
        <div
          className="space-y-4"
          onTouchStart={(e) => {
            touchX.current = e.changedTouches[0]?.clientX ?? null;
          }}
          onTouchEnd={(e) => {
            const start = touchX.current;
            const end = e.changedTouches[0]?.clientX;
            if (start == null || end == null) return;
            const d = end - start;
            if (d > 70) go(-1);
            if (d < -70) go(1);
          }}
        >
          <div className="flex flex-wrap items-center gap-2">
            <ModuleButton variant="secondary" onClick={() => setClasse(null)}>
              ← Classes
            </ModuleButton>
            <p className={`text-sm font-semibold ${dash.ink}`}>
              {SECTEUR_LABEL[classe.secteur]} · {classe.classe}
            </p>
            <p className={`text-sm ${dash.textMid}`}>
              {roster.length ? `${index + 1} / ${roster.length}` : "0"}
              {current ? ` — ${current.nom} ${current.prenom}` : ""}
            </p>
            <div className="ml-auto flex gap-2">
              <ModuleButton variant="secondary" onClick={() => go(-1)} disabled={index <= 0}>
                Précédent
              </ModuleButton>
              <ModuleButton variant="secondary" onClick={() => go(1)} disabled={index >= roster.length - 1}>
                Suivant
              </ModuleButton>
            </div>
          </div>

          {current ? (
            <div className="flex flex-wrap gap-1.5">
              {current.emptyDossier ? (
                <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-semibold text-slate-600">
                  Dossier vide
                </span>
              ) : null}
              {current.hasBulletin ? (
                <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[11px] font-semibold text-emerald-800">
                  Bulletin
                </span>
              ) : (
                <span className="rounded-full bg-amber-50 px-2 py-0.5 text-[11px] font-semibold text-amber-800">
                  Pas de bulletin
                </span>
              )}
              {current.hasPapPaiPps ? (
                <span className="rounded-full bg-violet-50 px-2 py-0.5 text-[11px] font-semibold text-violet-800">
                  PAP / PAI / PPS
                </span>
              ) : null}
              {current.dropSignal ? (
                <span className="rounded-full bg-red-50 px-2 py-0.5 text-[11px] font-semibold text-red-700">
                  Chute
                </span>
              ) : null}
            </div>
          ) : null}

          {loadingFiche ? (
            <p className={`text-sm ${dash.textMid}`}>Ouverture de la fiche…</p>
          ) : fiche ? (
            <FicheView
              fiche={fiche}
              canIndex={Boolean(overview?.canIndex)}
              canWriteNotes={Boolean(overview?.canWriteNotes)}
              indexing={indexing}
              onIndex={indexCurrent}
              od={od}
            />
          ) : (
            <p className={`text-sm ${dash.textMid}`}>Choisissez un élève.</p>
          )}
        </div>
      )}
    </ModulePageShell>
  );
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

  return (
    <div className="space-y-4">
      <section className="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
        <h2 className={`text-xl font-semibold ${dash.ink}`}>
          {fiche.eleve.nom} {fiche.eleve.prenom}
        </h2>
        <p className={`mt-1 text-sm ${dash.textMid}`}>
          {fiche.eleve.classe || "Sans classe"} · source : documents classés dans le dossier élève
        </p>
        {d?.drop ? <p className={`mt-2 text-sm ${dash.ink}`}>{d.drop.detail}</p> : null}
        {canIndex ? (
          <div className="mt-3">
            <ModuleButton onClick={onIndex} disabled={indexing}>
              {indexing ? "Indexation…" : "Actualiser depuis OneDrive (secrétariat)"}
            </ModuleButton>
            {od.error ? <p className="mt-1 text-xs text-red-600">{od.error}</p> : null}
            {!od.connected ? (
              <p className={`mt-1 text-xs ${dash.textMid}`}>Connexion Microsoft du secrétariat requise.</p>
            ) : null}
          </div>
        ) : null}
      </section>

      <section className="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
        <h3 className={`mb-2 font-semibold ${dash.ink}`}>Bulletins</h3>
        {bulletins.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>Aucun bulletin extrait pour l’instant.</p>
        ) : (
          <ul className="space-y-2">
            {bulletins.map((b) => (
              <li key={b.pieceId} className="rounded-xl bg-slate-50 px-3 py-2 text-sm">
                <p className="font-semibold">
                  {[b.anneeScolaire, b.periode, b.classe].filter(Boolean).join(" · ") || b.sourceName}
                </p>
                {typeof b.moyenneGenerale === "number" ? (
                  <p>Moyenne générale : {b.moyenneGenerale.toFixed(1)}</p>
                ) : null}
                {b.absencesMention ? <p className={dash.textMid}>Absences (bulletin) : {b.absencesMention}</p> : null}
                {b.appreciation ? <p className={dash.textMid}>{b.appreciation}</p> : null}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
        <h3 className={`mb-2 font-semibold ${dash.ink}`}>Pièces du dossier</h3>
        {!d || d.pieces.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>Pas encore de pièces indexées.</p>
        ) : (
          <ul className="text-sm">
            {d.pieces.map((p) => (
              <li key={p.id} className="flex justify-between gap-2 py-1">
                <span className="truncate">{p.name}</span>
                <span className={`shrink-0 uppercase ${dash.textMid}`}>{p.kind}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
        <h3 className={`mb-2 font-semibold ${dash.ink}`}>Synthèse documentaire (IA)</h3>
        <p className={`whitespace-pre-wrap text-sm leading-relaxed ${dash.ink}`}>
          {d?.synthese?.text || "La synthèse apparaîtra après le classement d’un document par le secrétariat."}
        </p>
        {d?.synthese?.sources?.length ? (
          <p className={`mt-2 text-xs ${dash.textMid}`}>Sources : {d.synthese.sources.join(" · ")}</p>
        ) : null}
      </section>

      {canWriteNotes ? (
        <PilotageDirectionNotes
          secteur={fiche.eleve.secteur}
          classe={fiche.eleve.classe}
          folderName={fiche.eleve.folderName}
          eleveLabel={`${fiche.eleve.nom} ${fiche.eleve.prenom}`}
          od={od}
        />
      ) : null}
    </div>
  );
}

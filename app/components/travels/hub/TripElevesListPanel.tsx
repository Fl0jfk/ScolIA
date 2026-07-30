"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  applyParticipantElevesToTripData,
  parentEmailCoverage,
  toParticipantEleve,
} from "@/app/lib/travels-eleves-list";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";

type Props = {
  trip: TravelsTrip;
  canEdit: boolean;
  onTripUpdated: (trip: TravelsTrip) => void;
};

export function TripElevesListPanel({ trip, canEdit, onTripUpdated }: Props) {
  const [eleves, setEleves] = useState<EleveConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedClasses, setSelectedClasses] = useState<string[]>([]);
  const [selectedInes, setSelectedInes] = useState<Set<string>>(new Set());
  const [droitByIne, setDroitByIne] = useState<Record<string, boolean>>({});
  const [busy, setBusy] = useState<"save" | "confirm" | null>(null);
  const [classFilter, setClassFilter] = useState("");

  const needsBus = complexNeedsBus(trip);
  const confirmed = trip.data.listeElevesStatus === "confirmed";

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    fetch("/api/eleves")
      .then(async (r) => {
        const j = await r.json();
        if (!r.ok) throw new Error(j.error || "Chargement élèves impossible");
        if (!cancelled) setEleves(Array.isArray(j.eleves) ? j.eleves : []);
      })
      .catch((e) => {
        if (!cancelled) setLoadError(e instanceof Error ? e.message : "Erreur");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const existing = trip.data.participantEleves || [];
    const ines = new Set(existing.map((p) => p.ine));
    setSelectedInes(ines);
    const droits: Record<string, boolean> = {};
    for (const p of existing) droits[p.ine] = p.droitImageOk !== false;
    setDroitByIne(droits);
    const classes = [...new Set(existing.map((p) => p.classe).filter(Boolean) as string[])];
    if (classes.length) setSelectedClasses(classes);
  }, [trip.id, trip.data.participantEleves, trip.data.listeElevesStatus]);

  const elevesByIne = useMemo(() => new Map(eleves.map((e) => [e.ine, e])), [eleves]);

  const allClasses = useMemo(() => {
    const set = new Set<string>();
    for (const e of eleves) {
      if (e.classe?.trim()) set.add(e.classe.trim());
    }
    return [...set].sort((a, b) => a.localeCompare(b, "fr"));
  }, [eleves]);

  const filteredClasses = useMemo(() => {
    const q = classFilter.trim().toLowerCase();
    if (!q) return allClasses;
    return allClasses.filter((c) => c.toLowerCase().includes(q));
  }, [allClasses, classFilter]);

  const elevesInSelectedClasses = useMemo(() => {
    if (selectedClasses.length === 0) return [];
    const classSet = new Set(selectedClasses);
    return eleves
      .filter((e) => e.classe && classSet.has(e.classe))
      .sort((a, b) =>
        `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
      );
  }, [eleves, selectedClasses]);

  const buildParticipants = useCallback((): TravelsParticipantEleve[] => {
    const list: TravelsParticipantEleve[] = [];
    for (const ine of selectedInes) {
      const full = elevesByIne.get(ine);
      if (full) {
        list.push(toParticipantEleve(full, droitByIne[ine] !== false));
      } else {
        const snap = (trip.data.participantEleves || []).find((p) => p.ine === ine);
        if (snap) list.push({ ...snap, droitImageOk: droitByIne[ine] !== false });
      }
    }
    return list.sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    );
  }, [selectedInes, elevesByIne, droitByIne, trip.data.participantEleves]);

  const coverage = useMemo(() => {
    const participants = buildParticipants();
    return parentEmailCoverage(participants, elevesByIne);
  }, [buildParticipants, elevesByIne]);

  const toggleClass = (classe: string) => {
    setSelectedClasses((prev) =>
      prev.includes(classe) ? prev.filter((c) => c !== classe) : [...prev, classe],
    );
  };

  const selectAllInClasses = () => {
    const next = new Set(selectedInes);
    const droits = { ...droitByIne };
    for (const e of elevesInSelectedClasses) {
      next.add(e.ine);
      if (droits[e.ine] === undefined) droits[e.ine] = true;
    }
    setSelectedInes(next);
    setDroitByIne(droits);
  };

  const clearSelectionInClasses = () => {
    const classSet = new Set(selectedClasses);
    const next = new Set(selectedInes);
    for (const e of eleves) {
      if (e.classe && classSet.has(e.classe)) next.delete(e.ine);
    }
    setSelectedInes(next);
  };

  const toggleEleve = (ine: string) => {
    setSelectedInes((prev) => {
      const next = new Set(prev);
      if (next.has(ine)) next.delete(ine);
      else {
        next.add(ine);
        setDroitByIne((d) => (d[ine] === undefined ? { ...d, [ine]: true } : d));
      }
      return next;
    });
  };

  const saveDraft = async () => {
    if (!canEdit) return;
    const participants = buildParticipants();
    setBusy("save");
    try {
      const data = applyParticipantElevesToTripData(trip.data, participants, {
        resetConfirmation: confirmed,
      });
      if (!data.listeElevesStatus) data.listeElevesStatus = "draft";
      const updatedTrip: TravelsTrip = {
        ...trip,
        data,
        history: [
          ...(trip.history || []),
          {
            date: new Date().toISOString(),
            user: "Utilisateur",
            action: `Liste élèves enregistrée (brouillon, ${participants.length} élève(s))`,
          },
        ],
      };
      const res = await fetch("/api/travels/update", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: trip.id, data: updatedTrip }),
      });
      if (!res.ok) {
        const j = await res.json().catch(() => ({}));
        throw new Error(j.error || "Enregistrement impossible");
      }
      onTripUpdated(updatedTrip);
      alert("Liste enregistrée (brouillon).");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  };

  const confirmList = async () => {
    if (!canEdit) return;
    const participants = buildParticipants();
    if (participants.length === 0) {
      return alert("Sélectionnez au moins un élève.");
    }
    const msg = needsBus
      ? `Confirmer la liste de ${participants.length} élève(s) et l'envoyer au transporteur ?`
      : `Confirmer la liste de ${participants.length} élève(s) ?`;
    if (!confirm(msg)) return;
    setBusy("confirm");
    try {
      const res = await fetch("/api/travels/confirm-eleves-list", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id, participantEleves: participants }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Confirmation impossible");
      onTripUpdated(j.trip as TravelsTrip);
      if (j.transportSkippedReason) alert(`Liste confirmée. ${j.transportSkippedReason}`);
      else if (j.sentTo?.length) alert(`Liste confirmée et envoyée à : ${j.sentTo.join(", ")}`);
      else alert("Liste confirmée.");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  };

  return (
    <TripSection
      title="Liste des élèves"
      subtitle="Sélection par classes depuis le fichier élèves de l'établissement"
      icon="👥"
      accent="indigo"
      action={
        confirmed ? (
          <span className="text-xs font-bold uppercase tracking-wide text-emerald-700 bg-emerald-50 px-3 py-1.5 rounded-full ring-1 ring-emerald-200">
            Confirmée
          </span>
        ) : (
          <span className="text-xs font-bold uppercase tracking-wide text-amber-800 bg-amber-50 px-3 py-1.5 rounded-full ring-1 ring-amber-200">
            Brouillon
          </span>
        )
      }
    >
      <div className="px-6 py-5 space-y-5">
        <TripAlert tone="info" icon="📷" title="Droit à l’image">
          Le droit à l’image est géré en interne par l’établissement. Par défaut, chaque élève
          sélectionné est marqué « OK » pour cette sortie (responsabilité établissement). Vous
          pouvez décocher élève par élève si besoin.
        </TripAlert>

        {needsBus && !confirmed && (
          <TripAlert tone="warning" icon="🚌" title="Transport bus">
            La confirmation de la liste est obligatoire pour le bus : elle déclenche l’envoi
            automatique au transporteur.
          </TripAlert>
        )}

        {trip.data.listeEnvoyeeTransporteurAt && (
          <p className="text-xs text-slate-500">
            Liste envoyée au transporteur le{" "}
            {new Date(trip.data.listeEnvoyeeTransporteurAt).toLocaleString("fr-FR")}.
          </p>
        )}

        {loading && <p className="text-sm text-slate-500">Chargement des élèves…</p>}
        {loadError && <TripAlert tone="warning" title="Erreur">{loadError}</TripAlert>}

        {!loading && !loadError && (
          <>
            <div>
              <label className="block text-xs font-semibold text-slate-500 uppercase tracking-wide mb-2">
                Classes
              </label>
              <input
                type="search"
                value={classFilter}
                onChange={(e) => setClassFilter(e.target.value)}
                placeholder="Filtrer une classe…"
                className="mb-2 w-full max-w-xs rounded-lg border border-slate-200 px-3 py-2 text-sm"
                disabled={!canEdit}
              />
              <div className="flex flex-wrap gap-2 max-h-36 overflow-y-auto">
                {filteredClasses.map((c) => {
                  const on = selectedClasses.includes(c);
                  return (
                    <button
                      key={c}
                      type="button"
                      disabled={!canEdit}
                      onClick={() => toggleClass(c)}
                      className={`text-xs px-3 py-1.5 rounded-full ring-1 transition ${
                        on
                          ? "bg-indigo-600 text-white ring-indigo-600"
                          : "bg-white text-slate-700 ring-slate-200 hover:bg-slate-50"
                      } disabled:opacity-50`}
                    >
                      {c}
                    </button>
                  );
                })}
                {filteredClasses.length === 0 && (
                  <p className="text-sm text-slate-500">Aucune classe dans le fichier élèves.</p>
                )}
              </div>
            </div>

            {selectedClasses.length > 0 && (
              <div>
                <div className="flex flex-wrap items-center gap-2 mb-3">
                  <p className="text-xs font-semibold text-slate-500 uppercase tracking-wide mr-auto">
                    Élèves ({selectedInes.size} sélectionné{selectedInes.size > 1 ? "s" : ""})
                  </p>
                  {canEdit && (
                    <>
                      <TripButton variant="secondary" size="sm" onClick={selectAllInClasses}>
                        Tout sélectionner
                      </TripButton>
                      <TripButton variant="secondary" size="sm" onClick={clearSelectionInClasses}>
                        Tout désélectionner
                      </TripButton>
                    </>
                  )}
                </div>
                <ul className="divide-y divide-slate-100 border border-slate-100 rounded-xl max-h-80 overflow-y-auto">
                  {elevesInSelectedClasses.map((e) => {
                    const checked = selectedInes.has(e.ine);
                    return (
                      <li key={e.ine} className="flex items-center gap-3 px-3 py-2 text-sm">
                        <input
                          type="checkbox"
                          checked={checked}
                          disabled={!canEdit}
                          onChange={() => toggleEleve(e.ine)}
                          className="rounded border-slate-300"
                        />
                        <span className="flex-1 min-w-0">
                          <span className="font-medium text-slate-800">
                            {e.nom} {e.prenom}
                          </span>
                          <span className="text-slate-400 ml-2 text-xs">{e.classe}</span>
                        </span>
                        {checked && (
                          <label className="flex items-center gap-1.5 text-xs text-slate-600 shrink-0">
                            <input
                              type="checkbox"
                              checked={droitByIne[e.ine] !== false}
                              disabled={!canEdit}
                              onChange={(ev) =>
                                setDroitByIne((d) => ({ ...d, [e.ine]: ev.target.checked }))
                              }
                            />
                            Droit image OK
                          </label>
                        )}
                      </li>
                    );
                  })}
                </ul>
              </div>
            )}

            <div className="rounded-xl bg-slate-50 border border-slate-100 px-4 py-3 text-sm text-slate-600">
              <p>
                Couverture mails parents :{" "}
                <strong className="text-slate-800">{coverage.withMail}</strong> avec e-mail ·{" "}
                <strong className="text-slate-800">{coverage.withoutMail}</strong> sans ·{" "}
                <strong className="text-slate-800">{coverage.emails.length}</strong> destinataire
                {coverage.emails.length > 1 ? "s" : ""} unique
                {coverage.emails.length > 1 ? "s" : ""}.
              </p>
              {coverage.withoutMail > 0 && (
                <p className="text-xs text-amber-700 mt-1">
                  Certains élèves n’ont pas d’e-mail parent renseigné — la communication ne les
                  atteindra pas.
                </p>
              )}
            </div>

            {canEdit && (
              <div className="flex flex-wrap gap-2 pt-1">
                <TripButton variant="secondary" disabled={!!busy} onClick={saveDraft}>
                  {busy === "save" ? "…" : "Enregistrer brouillon"}
                </TripButton>
                <TripButton variant="primary" disabled={!!busy || selectedInes.size === 0} onClick={confirmList}>
                  {busy === "confirm" ? "…" : needsBus ? "Confirmer et envoyer au transporteur" : "Confirmer la liste"}
                </TripButton>
              </div>
            )}
          </>
        )}
      </div>
    </TripSection>
  );
}

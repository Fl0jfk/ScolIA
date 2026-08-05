"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  applyParticipantElevesToTripData,
  eleveParticipantKey,
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
  /** Classe en cours de parcours (pour ajouter des élèves). */
  const [browseClass, setBrowseClass] = useState<string | null>(null);
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [droitByKey, setDroitByKey] = useState<Record<string, boolean>>({});
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
    const keys = new Set(existing.map((p) => eleveParticipantKey(p)));
    setSelectedKeys(keys);
    const droits: Record<string, boolean> = {};
    for (const p of existing) droits[eleveParticipantKey(p)] = p.droitImageOk !== false;
    setDroitByKey(droits);
    const classes = [...new Set(existing.map((p) => p.classe).filter(Boolean) as string[])];
    if (classes.length && !browseClass) setBrowseClass(classes[0]!);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- sync from trip only
  }, [trip.id, trip.data.participantEleves, trip.data.listeElevesStatus]);

  const elevesByKey = useMemo(() => {
    const map = new Map<string, EleveConfig>();
    for (const e of eleves) map.set(eleveParticipantKey(e), e);
    return map;
  }, [eleves]);

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

  const elevesInBrowseClass = useMemo(() => {
    if (!browseClass) return [];
    return eleves
      .filter((e) => e.classe === browseClass)
      .sort((a, b) =>
        `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
      );
  }, [eleves, browseClass]);

  const selectedParticipants = useMemo(() => {
    const list: Array<{ key: string; eleve: EleveConfig | TravelsParticipantEleve }> = [];
    for (const key of selectedKeys) {
      const full = elevesByKey.get(key);
      if (full) list.push({ key, eleve: full });
      else {
        const snap = (trip.data.participantEleves || []).find(
          (p) => eleveParticipantKey(p) === key,
        );
        if (snap) list.push({ key, eleve: snap });
      }
    }
    return list.sort((a, b) =>
      `${a.eleve.nom} ${a.eleve.prenom}`.localeCompare(
        `${b.eleve.nom} ${b.eleve.prenom}`,
        "fr",
        { sensitivity: "base" },
      ),
    );
  }, [selectedKeys, elevesByKey, trip.data.participantEleves]);

  const classesInList = useMemo(() => {
    const set = new Set<string>();
    for (const { eleve } of selectedParticipants) {
      if (eleve.classe?.trim()) set.add(eleve.classe.trim());
    }
    return [...set].sort((a, b) => a.localeCompare(b, "fr"));
  }, [selectedParticipants]);

  const buildParticipants = useCallback((): TravelsParticipantEleve[] => {
    const list: TravelsParticipantEleve[] = [];
    for (const key of selectedKeys) {
      const full = elevesByKey.get(key);
      if (full) {
        list.push(toParticipantEleve(full, droitByKey[key] !== false));
      } else {
        const snap = (trip.data.participantEleves || []).find(
          (p) => eleveParticipantKey(p) === key,
        );
        if (snap) {
          list.push({
            ...snap,
            ine: eleveParticipantKey(snap),
            droitImageOk: droitByKey[key] !== false,
          });
        }
      }
    }
    return list.sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    );
  }, [selectedKeys, elevesByKey, droitByKey, trip.data.participantEleves]);

  const coverage = useMemo(() => {
    const participants = buildParticipants();
    return parentEmailCoverage(participants, elevesByKey);
  }, [buildParticipants, elevesByKey]);

  const countSelectedInClass = (classe: string) => {
    let n = 0;
    for (const e of eleves) {
      if (e.classe === classe && selectedKeys.has(eleveParticipantKey(e))) n += 1;
    }
    return n;
  };

  const addEleveToList = (e: EleveConfig) => {
    if (!canEdit) return;
    const key = eleveParticipantKey(e);
    setSelectedKeys((prev) => new Set(prev).add(key));
    setDroitByKey((d) => (d[key] === undefined ? { ...d, [key]: true } : d));
  };

  const removeEleveFromList = (key: string) => {
    if (!canEdit) return;
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      next.delete(key);
      return next;
    });
  };

  const toggleEleveInList = (e: EleveConfig) => {
    const key = eleveParticipantKey(e);
    if (selectedKeys.has(key)) removeEleveFromList(key);
    else addEleveToList(e);
  };

  const addWholeClassToList = (classe: string) => {
    if (!canEdit) return;
    const next = new Set(selectedKeys);
    const droits = { ...droitByKey };
    for (const e of eleves) {
      if (e.classe !== classe) continue;
      const key = eleveParticipantKey(e);
      next.add(key);
      if (droits[key] === undefined) droits[key] = true;
    }
    setSelectedKeys(next);
    setDroitByKey(droits);
  };

  const removeWholeClassFromList = (classe: string) => {
    if (!canEdit) return;
    const next = new Set(selectedKeys);
    for (const e of eleves) {
      if (e.classe === classe) next.delete(eleveParticipantKey(e));
    }
    setSelectedKeys(next);
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
      return alert("Ajoutez au moins un élève à la liste.");
    }
    const msg = needsBus
      ? `Confirmer la liste de ${participants.length} élève(s) et l'envoyer au transporteur (CSV : nom, prénom, classe, mail et tél. parents) ?`
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
      if (j.trip) onTripUpdated(j.trip as TravelsTrip);
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
      subtitle="Ajoutez des élèves de plusieurs classes — la liste s’accumule"
      icon="👥"
      accent="indigo"
      action={
        confirmed ? (
          <span className="text-xs font-bold uppercase tracking-wide text-emerald-700 bg-emerald-50 px-3 py-1.5 rounded-full ring-1 ring-emerald-200">
            Confirmée
          </span>
        ) : (
          <span className="text-xs font-bold uppercase tracking-wide text-amber-800 bg-amber-50 px-3 py-1.5 rounded-full ring-1 ring-amber-200">
            Brouillon · {selectedKeys.size} élève{selectedKeys.size > 1 ? "s" : ""}
          </span>
        )
      }
    >
      <div className="px-6 py-5 space-y-5">
        {!canEdit && (
          <TripAlert tone="warning" icon="🔒" title="Lecture seule">
            Vous ne pouvez pas modifier cette liste (réservé au créateur de la sortie, à la
            direction ou à l&apos;administratif).
          </TripAlert>
        )}

        <TripAlert tone="info" icon="📷" title="Droit à l’image">
          Par défaut, chaque élève ajouté est marqué « OK » pour le droit à l’image. Vous pouvez
          décocher élève par élève dans la liste.
        </TripAlert>

        {needsBus && !confirmed && (
          <TripAlert tone="warning" icon="🚌" title="Transport bus">
            À la confirmation, le transporteur reçoit un CSV : Nom, Prénom, Classe, Email parent,
            Tél. parent.
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
            {/* Parcourir / ajouter par classe */}
            <div>
              <label className="mb-2 block text-xs font-semibold uppercase tracking-wide text-slate-500">
                1 — Choisir une classe pour ajouter des élèves
              </label>
              <input
                type="search"
                value={classFilter}
                onChange={(e) => setClassFilter(e.target.value)}
                placeholder="Filtrer une classe…"
                className="mb-2 w-full max-w-xs rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
              <div className="flex max-h-36 flex-wrap gap-2 overflow-y-auto">
                {filteredClasses.map((c) => {
                  const active = browseClass === c;
                  const n = countSelectedInClass(c);
                  return (
                    <button
                      key={c}
                      type="button"
                      onClick={() => setBrowseClass(c)}
                      className={`rounded-full px-3 py-1.5 text-xs ring-1 transition ${
                        active
                          ? "bg-indigo-600 text-white ring-indigo-600"
                          : n > 0
                            ? "bg-indigo-50 text-indigo-900 ring-indigo-200"
                            : "bg-white text-slate-700 ring-slate-200 hover:bg-slate-50"
                      }`}
                    >
                      {c}
                      {n > 0 ? ` · ${n}` : ""}
                    </button>
                  );
                })}
                {filteredClasses.length === 0 && (
                  <p className="text-sm text-slate-500">Aucune classe dans le fichier élèves.</p>
                )}
              </div>
            </div>

            {browseClass && (
              <div className="rounded-xl border border-slate-200 bg-white p-3">
                <div className="mb-3 flex flex-wrap items-center gap-2">
                  <p className="mr-auto text-xs font-semibold uppercase tracking-wide text-slate-500">
                    Classe {browseClass}
                  </p>
                  {canEdit && (
                    <>
                      <TripButton
                        variant="secondary"
                        size="sm"
                        onClick={() => addWholeClassToList(browseClass)}
                      >
                        + Ajouter toute la classe
                      </TripButton>
                      {countSelectedInClass(browseClass) > 0 && (
                        <TripButton
                          variant="secondary"
                          size="sm"
                          onClick={() => removeWholeClassFromList(browseClass)}
                        >
                          Retirer cette classe
                        </TripButton>
                      )}
                    </>
                  )}
                </div>
                <ul className="max-h-56 divide-y divide-slate-100 overflow-y-auto rounded-lg border border-slate-100">
                  {elevesInBrowseClass.map((e) => {
                    const key = eleveParticipantKey(e);
                    const inList = selectedKeys.has(key);
                    return (
                      <li key={key} className="flex items-center gap-3 px-3 py-2 text-sm">
                        <input
                          type="checkbox"
                          checked={inList}
                          disabled={!canEdit}
                          onChange={() => toggleEleveInList(e)}
                          className="rounded border-slate-300"
                        />
                        <span className="min-w-0 flex-1 font-medium text-slate-800">
                          {e.nom} {e.prenom}
                        </span>
                        {inList ? (
                          <span className="text-[10px] font-bold uppercase text-emerald-700">
                            Dans la liste
                          </span>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              </div>
            )}

            {/* Liste accumulée */}
            <div className="rounded-xl border border-indigo-200 bg-indigo-50/40 p-4">
              <div className="mb-3 flex flex-wrap items-center gap-2">
                <h3 className="text-sm font-black text-indigo-950">
                  2 — Liste pour la sortie ({selectedKeys.size})
                </h3>
                {classesInList.length > 0 && (
                  <p className="text-[11px] text-indigo-800/80">
                    Classes : {classesInList.join(", ")}
                  </p>
                )}
              </div>

              {selectedParticipants.length === 0 ? (
                <p className="text-sm text-indigo-900/60">
                  Aucun élève pour l’instant. Ouvrez une classe ci-dessus et ajoutez des élèves —
                  vous pouvez enchaîner plusieurs classes.
                </p>
              ) : (
                <ul className="max-h-72 divide-y divide-indigo-100 overflow-y-auto rounded-xl border border-indigo-100 bg-white">
                  {selectedParticipants.map(({ key, eleve }) => (
                    <li key={key} className="flex items-center gap-3 px-3 py-2 text-sm">
                      <span className="min-w-0 flex-1">
                        <span className="font-medium text-slate-800">
                          {eleve.nom} {eleve.prenom}
                        </span>
                        <span className="ml-2 text-xs text-slate-400">{eleve.classe}</span>
                      </span>
                      <label className="flex shrink-0 items-center gap-1.5 text-xs text-slate-600">
                        <input
                          type="checkbox"
                          checked={droitByKey[key] !== false}
                          disabled={!canEdit}
                          onChange={(ev) =>
                            setDroitByKey((d) => ({ ...d, [key]: ev.target.checked }))
                          }
                        />
                        Droit image OK
                      </label>
                      {canEdit && (
                        <button
                          type="button"
                          onClick={() => removeEleveFromList(key)}
                          className="text-xs font-bold text-rose-600 hover:underline"
                        >
                          Retirer
                        </button>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>

            <div className="rounded-xl border border-slate-100 bg-slate-50 px-4 py-3 text-sm text-slate-600">
              <p>
                Couverture mails parents :{" "}
                <strong className="text-slate-800">{coverage.withMail}</strong> avec e-mail ·{" "}
                <strong className="text-slate-800">{coverage.withoutMail}</strong> sans ·{" "}
                <strong className="text-slate-800">{coverage.emails.length}</strong> destinataire
                {coverage.emails.length > 1 ? "s" : ""} unique
                {coverage.emails.length > 1 ? "s" : ""}.
              </p>
              {coverage.withoutMail > 0 && (
                <p className="mt-1 text-xs text-amber-700">
                  Certains élèves n’ont pas d’e-mail parent — le CSV transporteur aura la case
                  vide pour eux.
                </p>
              )}
            </div>

            {canEdit && (
              <div className="flex flex-wrap gap-2 pt-1">
                <TripButton variant="secondary" disabled={!!busy} onClick={saveDraft}>
                  {busy === "save" ? "…" : "Enregistrer brouillon"}
                </TripButton>
                <TripButton
                  variant="primary"
                  disabled={!!busy || selectedKeys.size === 0}
                  onClick={confirmList}
                >
                  {busy === "confirm"
                    ? "…"
                    : needsBus
                      ? "Confirmer et envoyer au transporteur"
                      : "Confirmer la liste"}
                </TripButton>
              </div>
            )}
          </>
        )}
      </div>
    </TripSection>
  );
}

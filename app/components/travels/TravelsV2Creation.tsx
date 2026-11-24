"use client";

/**
 * Blocs de création du circuit voyages v2 : rétroplanning, pré-liste provisoire des élèves invités,
 * éditeur des pièces demandées aux familles. Aucun prix n'est saisi ici (le prix vient de la compta).
 */

import { useEffect, useMemo, useState } from "react";
import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  compareParticipantsByClasseThenName,
  eleveParticipantKey,
  toParticipantEleve,
} from "@/app/lib/travels-eleves-list";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { uploadTravelDocument } from "@/app/lib/travels-upload-client";
import type {
  TravelsFamilyDocKind,
  TravelsFamilyDocRequest,
  TravelsParticipantEleve,
} from "@/app/lib/travels-types";
import {
  computeRetroplanning,
  HEALTH_DOC_FORBIDDEN_MESSAGE,
  validateFamilyDocuments,
} from "@/app/lib/travels-v2";

export const PRELISTE_LABEL = "Pré-liste provisoire — la liste officielle sera celle des familles qui auront signé";

const KIND_LABELS: Record<TravelsFamilyDocKind, string> = {
  avant_signature: "Obligatoire avant de signer",
  facultative: "Facultative",
  avant_depart: "À fournir avant le départ",
};

export function newFamilyDocId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `doc-${Date.now()}-${Math.round(Math.random() * 1e6)}`;
}

/* ─── Rétroplanning ─── */

export function RetroplanningCard({ departureIso, todayIso }: { departureIso: string; todayIso: string }) {
  const plan = useMemo(() => computeRetroplanning(departureIso, todayIso), [departureIso, todayIso]);
  if (!plan) {
    return (
      <p className="text-xs text-slate-500">
        Renseignez la date de départ pour afficher le rétroplanning des inscriptions.
      </p>
    );
  }
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-sm">
        <div className="rounded-2xl bg-slate-50 border p-3">
          <p className="text-[10px] font-bold uppercase text-slate-400">Validation finale au plus tard</p>
          <p className="font-bold text-slate-900">{plan.latestFinalValidationIso} <span className="text-slate-400 font-normal">(J-21)</span></p>
        </div>
        <div className="rounded-2xl bg-slate-50 border p-3">
          <p className="text-[10px] font-bold uppercase text-slate-400">Inscriptions ouvertes</p>
          <p className="font-bold text-slate-900">au moins 7 jours</p>
        </div>
        <div className="rounded-2xl bg-slate-50 border p-3">
          <p className="text-[10px] font-bold uppercase text-slate-400">Clôture des inscriptions au plus tard</p>
          <p className="font-bold text-slate-900">{plan.latestDeadlineIso} <span className="text-slate-400 font-normal">(J-14)</span></p>
        </div>
      </div>
      <p className="text-[11px] text-slate-500">
        J-14 = délai cuisine et devis rectifié. Départ le {plan.departureIso}.
      </p>
      {plan.warning ? (
        <div className="rounded-2xl border-2 border-red-300 bg-red-50 p-3 text-sm font-semibold text-red-800" role="alert">
          ⚠️ {plan.warning}
        </div>
      ) : null}
    </div>
  );
}

/* ─── Pré-liste des élèves invités ─── */

export function TravelsPreListePicker({
  value,
  onChange,
  tripClasses,
}: {
  value: TravelsParticipantEleve[];
  onChange: (next: TravelsParticipantEleve[]) => void;
  /** Classes déjà choisies pour le séjour (ouvertes en premier). */
  tripClasses?: string;
}) {
  const [eleves, setEleves] = useState<EleveConfig[]>([]);
  const [classes, setClasses] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [browseClass, setBrowseClass] = useState("");

  useEffect(() => {
    let cancelled = false;
    fetch("/api/travels/eleves-picker")
      .then(async (r) => {
        const j = await r.json();
        if (!r.ok) throw new Error(j.error || "Chargement des élèves impossible");
        if (cancelled) return;
        setEleves(Array.isArray(j.eleves) ? j.eleves : []);
        setClasses(Array.isArray(j.classes) ? j.classes : []);
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : "Erreur");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const tripClassList = useMemo(
    () => String(tripClasses || "").split(/[,;]/).map((c) => c.trim()).filter(Boolean),
    [tripClasses],
  );
  const orderedClasses = useMemo(() => {
    const first = classes.filter((c) => tripClassList.some((t) => schoolClassesMatch(c, t)));
    const rest = classes.filter((c) => !first.includes(c));
    return [...first, ...rest];
  }, [classes, tripClassList]);

  useEffect(() => {
    if (!browseClass && orderedClasses.length > 0) setBrowseClass(orderedClasses[0]!);
  }, [orderedClasses, browseClass]);

  const selectedKeys = useMemo(() => new Set(value.map((p) => eleveParticipantKey(p))), [value]);
  const inClass = useMemo(
    () =>
      eleves
        .filter((e) => browseClass && schoolClassesMatch(e.classe, browseClass))
        .sort((a, b) => `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" })),
    [eleves, browseClass],
  );

  const toggle = (e: EleveConfig) => {
    const key = eleveParticipantKey(e);
    if (selectedKeys.has(key)) onChange(value.filter((p) => eleveParticipantKey(p) !== key));
    else onChange([...value, toParticipantEleve(e)].sort(compareParticipantsByClasseThenName));
  };
  const allInClassSelected = inClass.length > 0 && inClass.every((e) => selectedKeys.has(eleveParticipantKey(e)));
  const toggleAllClass = () => {
    if (allInClassSelected) {
      const drop = new Set(inClass.map((e) => eleveParticipantKey(e)));
      onChange(value.filter((p) => !drop.has(eleveParticipantKey(p))));
    } else {
      const add = inClass.filter((e) => !selectedKeys.has(eleveParticipantKey(e))).map((e) => toParticipantEleve(e));
      onChange([...value, ...add].sort(compareParticipantsByClasseThenName));
    }
  };

  return (
    <div className="space-y-3">
      <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm font-semibold text-amber-900">
        {PRELISTE_LABEL}
      </div>
      {loading ? <p className="text-sm text-slate-500">Chargement des élèves…</p> : null}
      {error ? <p className="text-sm text-red-600">{error}</p> : null}
      {!loading && !error ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <label className="block text-xs font-bold">Classe</label>
            <select
              value={browseClass}
              onChange={(e) => setBrowseClass(e.target.value)}
              className="w-full p-2.5 bg-slate-50 border rounded-xl"
            >
              {orderedClasses.map((c) => (
                <option key={c} value={c}>{c}</option>
              ))}
            </select>
            <button type="button" onClick={toggleAllClass} className="text-xs font-bold text-indigo-600 hover:underline">
              {allInClassSelected ? "Retirer toute la classe" : "Inviter toute la classe"}
            </button>
            <div className="max-h-64 overflow-y-auto rounded-xl border divide-y bg-white">
              {inClass.map((e) => {
                const key = eleveParticipantKey(e);
                return (
                  <label key={key} className="flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer hover:bg-slate-50">
                    <input type="checkbox" checked={selectedKeys.has(key)} onChange={() => toggle(e)} />
                    <span>{e.nom} {e.prenom}</span>
                  </label>
                );
              })}
              {inClass.length === 0 ? <p className="px-3 py-2 text-xs text-slate-400">Aucun élève dans cette classe.</p> : null}
            </div>
          </div>
          <div>
            <p className="text-xs font-bold mb-2">Élèves invités : {value.length}</p>
            <div className="max-h-72 overflow-y-auto rounded-xl border bg-slate-50 p-2 text-sm space-y-0.5">
              {value.length === 0 ? <p className="text-xs text-slate-400 p-1">Aucun élève invité pour l’instant.</p> : null}
              {value.map((p) => (
                <div key={eleveParticipantKey(p)} className="flex items-center justify-between gap-2">
                  <span>{p.classe ? <span className="text-slate-400 text-xs mr-1">{p.classe}</span> : null}{p.nom} {p.prenom}</span>
                  <button
                    type="button"
                    className="text-red-500 text-xs"
                    onClick={() => onChange(value.filter((x) => eleveParticipantKey(x) !== eleveParticipantKey(p)))}
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

/* ─── Éditeur de pièces demandées ─── */

export function FamilyDocumentsEditor({
  value,
  onChange,
  departureIso,
  disabled = false,
}: {
  value: TravelsFamilyDocRequest[];
  onChange: (next: TravelsFamilyDocRequest[]) => void;
  departureIso?: string | null;
  disabled?: boolean;
}) {
  const [label, setLabel] = useState("");
  const [kind, setKind] = useState<TravelsFamilyDocKind>("avant_signature");
  const [dueDate, setDueDate] = useState("");
  const [template, setTemplate] = useState<TravelsFamilyDocRequest["template"] | null>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const addDoc = () => {
    const doc: TravelsFamilyDocRequest = {
      id: newFamilyDocId(),
      label: label.trim(),
      kind,
      ...(kind === "avant_depart" ? { dueDate } : {}),
      ...(template ? { template } : {}),
    };
    const res = validateFamilyDocuments([doc], { departureIso });
    if (!res.ok) {
      setError(res.error);
      return;
    }
    setError(null);
    onChange([...value, doc]);
    setLabel("");
    setDueDate("");
    setTemplate(null);
  };

  const uploadTemplate = async (file: File | undefined) => {
    if (!file) return;
    setUploading(true);
    try {
      const { fileUrl, s3Key } = await uploadTravelDocument(file, file.name);
      setTemplate({ name: file.name, url: fileUrl, s3Key });
    } catch (e) {
      console.error(e);
      setError("Impossible d'envoyer le modèle.");
    } finally {
      setUploading(false);
    }
  };

  return (
    <div className="space-y-3">
      <div className="rounded-2xl border-2 border-rose-200 bg-rose-50 px-4 py-3 text-sm font-semibold text-rose-900" role="note">
        🚫 {HEALTH_DOC_FORBIDDEN_MESSAGE}
      </div>
      <ul className="space-y-2">
        {value.map((d) => (
          <li key={d.id} className="flex flex-wrap items-center justify-between gap-2 rounded-xl border bg-white px-3 py-2 text-sm">
            <span>
              <span className="font-bold">{d.label}</span>{" "}
              <span className="text-slate-500">— {KIND_LABELS[d.kind]}{d.kind === "avant_depart" && d.dueDate ? ` (au plus tard le ${d.dueDate})` : ""}</span>
              {d.template ? <span className="ml-2 text-xs text-indigo-600">modèle : {d.template.name}</span> : null}
            </span>
            {!disabled ? (
              <button type="button" className="text-red-500 text-xs font-bold" onClick={() => onChange(value.filter((x) => x.id !== d.id))}>
                Retirer
              </button>
            ) : null}
          </li>
        ))}
        {value.length === 0 ? <li className="text-xs text-slate-400">Aucune pièce demandée pour l’instant.</li> : null}
      </ul>
      {!disabled ? (
        <div className="rounded-2xl border bg-slate-50 p-3 space-y-3">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div className="md:col-span-2">
              <label className="block text-xs font-bold mb-1">Intitulé de la pièce</label>
              <input
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder="Ex : Attestation d'aisance aquatique"
                className="w-full p-2.5 bg-white border rounded-xl"
              />
            </div>
            <div>
              <label className="block text-xs font-bold mb-1">Type</label>
              <select value={kind} onChange={(e) => setKind(e.target.value as TravelsFamilyDocKind)} className="w-full p-2.5 bg-white border rounded-xl">
                {(Object.keys(KIND_LABELS) as TravelsFamilyDocKind[]).map((k) => (
                  <option key={k} value={k}>{KIND_LABELS[k]}</option>
                ))}
              </select>
            </div>
            {kind === "avant_depart" ? (
              <div>
                <label className="block text-xs font-bold mb-1">À fournir au plus tard le</label>
                <input type="date" value={dueDate} max={departureIso || undefined} onChange={(e) => setDueDate(e.target.value)} className="w-full p-2.5 bg-white border rounded-xl" />
              </div>
            ) : null}
            <div className="md:col-span-2">
              <label className="block text-xs font-bold mb-1">Modèle vierge à télécharger (facultatif)</label>
              <input type="file" disabled={uploading} onChange={(e) => void uploadTemplate(e.target.files?.[0])} className="text-xs" />
              {template ? <p className="text-xs text-emerald-700 mt-1">✅ {template.name}</p> : null}
            </div>
          </div>
          {error ? <p className="text-sm font-semibold text-red-700" role="alert">{error}</p> : null}
          <button
            type="button"
            onClick={addDoc}
            disabled={!label.trim() || uploading}
            className="px-4 py-2 rounded-xl bg-indigo-600 text-white text-sm font-bold disabled:opacity-40"
          >
            Ajouter la pièce
          </button>
        </div>
      ) : null}
    </div>
  );
}

/* ─── Section complète pour les formulaires de création ─── */

export function TravelsV2CreationSection({
  departureIso,
  todayIso,
  tripClasses,
  preListe,
  onPreListeChange,
  documents,
  onDocumentsChange,
  stepNumber,
}: {
  departureIso: string;
  todayIso: string;
  tripClasses?: string;
  preListe: TravelsParticipantEleve[];
  onPreListeChange: (next: TravelsParticipantEleve[]) => void;
  documents: TravelsFamilyDocRequest[];
  onDocumentsChange: (next: TravelsFamilyDocRequest[]) => void;
  stepNumber: number;
}) {
  return (
    <div className="space-y-6">
      <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-4">
        <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">
          {stepNumber}. Inscriptions des familles — rétroplanning
        </div>
        <RetroplanningCard departureIso={departureIso} todayIso={todayIso} />
        <p className="text-xs text-slate-500">
          Aucun prix n’est saisi ici : le prix par élève et les échéances sont fixés par la comptabilité. La page des familles
          sera rédigée après la validation pédagogique.
        </p>
      </div>
      <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-4">
        <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">
          {stepNumber + 1}. Élèves invités
        </div>
        <TravelsPreListePicker value={preListe} onChange={onPreListeChange} tripClasses={tripClasses} />
      </div>
      <div className="bg-white p-8 border rounded-3xl shadow-sm space-y-4">
        <div className="text-slate-400 uppercase text-xs font-bold tracking-widest border-b pb-4">
          {stepNumber + 2}. Pièces demandées aux familles
        </div>
        <FamilyDocumentsEditor value={documents} onChange={onDocumentsChange} departureIso={departureIso} />
      </div>
    </div>
  );
}

"use client";

import Image from "next/image";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import type { OrganigramPerson } from "@/app/lib/organigramme";
import type { OrganigramView } from "@/app/lib/organigramme-resolve";
import { OrganigramServiceFrame, OrganigramPoleColumn } from "./OrganigramServiceFrame";
import { OrganigramPrintDocument } from "./OrganigramPrintDocument";
import { OrganigrammeEditor } from "./OrganigrammeEditor";

function poleVariantFor(id: string): "poleEcole" | "poleCollege" | "poleLycee" {
  if (id === "pole-ecole") return "poleEcole";
  if (id === "pole-college") return "poleCollege";
  return "poleLycee";
}

function initials(p: OrganigramPerson): string {
  const f = (p.firstName ?? "").trim();
  const l = (p.lastName ?? "").trim();
  if (f && l) return `${f[0]}${l[0]}`.toUpperCase();
  if (f) return f.slice(0, 2).toUpperCase();
  if (l) return l.slice(0, 2).toUpperCase();
  const r = p.role.replace(/[^a-zA-ZÀ-ÿ]/g, " ").trim();
  const parts = r.split(/\s+/).filter(Boolean);
  if (parts.length >= 2) return `${parts[0]![0]}${parts[1]![0]}`.toUpperCase();
  if (parts[0]?.length && parts[0].length >= 2) return parts[0].slice(0, 2).toUpperCase();
  return "?";
}

function displayName(p: OrganigramPerson): string {
  const f = (p.firstName ?? "").trim();
  const l = (p.lastName ?? "").trim();
  if (f || l) return [f, l].filter(Boolean).join(" ");
  return "À compléter";
}

function PersonCard({
  person,
  onSelect,
  compact,
}: {
  person: OrganigramPerson;
  onSelect: (p: OrganigramPerson) => void;
  compact?: boolean;
}) {
  const name = displayName(person);
  const ini = initials(person);
  return (
    <button
      type="button"
      onClick={() => onSelect(person)}
      className={`group text-left w-full rounded-2xl border border-slate-200/80 bg-white/90 shadow-sm hover:shadow-md hover:border-sky-300/60 transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 ${
        compact ? "p-3" : "p-4"
      }`}
    >
      <div className="flex gap-3 items-start">
        <div
          className={`shrink-0 rounded-xl overflow-hidden bg-gradient-to-br from-slate-100 to-slate-200 border border-slate-200 flex items-center justify-center text-slate-600 font-bold ${
            compact ? "w-12 h-12 text-sm" : "w-14 h-14 text-base"
          }`}
        >
          {person.photoUrl ? (
            <Image src={person.photoUrl} alt="" width={56} height={56} className="w-full h-full object-cover" />
          ) : (
            <span>{ini}</span>
          )}
        </div>
        <div className="min-w-0 flex-1">
          <p className={`font-semibold text-slate-900 truncate ${compact ? "text-sm" : "text-[15px]"}`}>
            {name}
          </p>
          <p className={`text-sky-800/90 font-medium leading-snug mt-0.5 ${compact ? "text-xs" : "text-sm"}`}>
            {person.role}
          </p>
          <p className="text-[11px] text-slate-400 mt-2 opacity-0 group-hover:opacity-100 transition-opacity">
            Voir les missions →
          </p>
        </div>
      </div>
    </button>
  );
}

function sectionMeta(view: OrganigramView, id: string) {
  return view.sections.find((s) => s.id === id);
}

export default function OrganigrammePageClient() {
  const searchParams = useSearchParams();
  const editMode = searchParams.get("edit") === "1";

  const [view, setView] = useState<OrganigramView | null>(null);
  const [canEdit, setCanEdit] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<OrganigramPerson | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/organigramme", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setView(data.view as OrganigramView);
      setCanEdit(Boolean(data.canEdit));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const modalMissions = useMemo(() => {
    if (!selected) return [];
    return selected.missions.map((m) => m.trim()).filter(Boolean);
  }, [selected]);

  if (loading) {
    return (
      <main className="min-h-screen flex items-center justify-center text-slate-500 text-sm">
        Chargement de l&apos;organigramme…
      </main>
    );
  }

  if (error || !view) {
    return (
      <main className="min-h-screen flex flex-col items-center justify-center gap-3 text-sm">
        <p className="text-red-600">{error || "Organigramme indisponible"}</p>
        <button
          type="button"
          onClick={() => void reload()}
          className="rounded-xl bg-slate-900 text-white px-4 py-2 font-semibold"
        >
          Réessayer
        </button>
      </main>
    );
  }

  const dir = sectionMeta(view, "direction");
  const polesMeta = sectionMeta(view, "poles");

  return (
    <main className="relative min-h-screen w-full max-w-6xl mx-auto px-4 sm:px-6 pb-16 pt-[4vh] overflow-x-clip print:max-w-none print:mx-0 print:px-4 print:pb-0 print:pt-2 print:overflow-visible">
      <div className="print:hidden">
        <header className="mb-10 flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
          <div>
            <h1 className="text-3xl sm:text-4xl font-black text-slate-900 tracking-tight">
              Organigramme interne
            </h1>
          </div>
          <div className="flex flex-wrap gap-2">
            {canEdit ? (
              <a
                href={editMode ? "/organigramme" : "/organigramme?edit=1"}
                className="shrink-0 self-start px-4 py-2.5 rounded-xl border border-slate-300 bg-white text-slate-800 text-sm font-bold hover:bg-slate-50"
              >
                {editMode ? "Quitter l'édition" : "Éditer"}
              </a>
            ) : null}
            <button
              type="button"
              onClick={() => window.print()}
              className="shrink-0 self-start px-5 py-3 rounded-xl bg-slate-900 text-white text-sm font-bold shadow-lg hover:bg-slate-800 transition-colors border border-slate-700"
            >
              Imprimer / PDF (A4)
            </button>
          </div>
        </header>

        {editMode && canEdit ? <OrganigrammeEditor onSaved={() => void reload()} /> : null}

        <div className="relative flex flex-col gap-4 sm:gap-6 md:gap-9 lg:gap-12">
          <OrganigramServiceFrame
            variant="direction"
            slotIndex={0}
            title={dir?.title || "Direction du groupe scolaire"}
            description={dir?.description}
          >
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              {view.directors.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={1}
            variant="admin"
            title={view.admin.title}
            description={view.admin.description}
          >
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              {view.admin.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={2}
            variant="accounting"
            title={view.accounting.title}
            description={view.accounting.description}
          >
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              {view.accounting.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={6}
            variant="poles"
            bareContent
            title={polesMeta?.title || "Pôles éducatifs & vie scolaire"}
            description={polesMeta?.description}
          >
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 w-full">
              {view.poles.map((pole) => (
                <OrganigramPoleColumn key={pole.id} poleVariant={poleVariantFor(pole.id)} label={pole.label}>
                  {pole.blocks.map((block) => (
                    <div key={block.id} className="mb-3 last:mb-0">
                      <p className="text-xs font-semibold text-slate-600 mb-1">{block.title}</p>
                      {block.description ? (
                        <p className="text-[11px] text-slate-500 mb-2">{block.description}</p>
                      ) : null}
                      <div className="space-y-2">
                        {block.people.map((p) => (
                          <PersonCard key={p.id} person={p} onSelect={setSelected} compact />
                        ))}
                      </div>
                    </div>
                  ))}
                </OrganigramPoleColumn>
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={3}
            variant="reception"
            title={view.reception.title}
            description={view.reception.description}
          >
            {view.reception.people.map((p) => (
              <PersonCard key={p.id} person={p} onSelect={setSelected} />
            ))}
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={4}
            variant="health"
            title={view.health.title}
            description={view.health.description}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 w-full">
              {view.health.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={5}
            variant="maintenance"
            title={view.maintenance.title}
            description={view.maintenance.description}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {view.maintenance.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={7}
            variant="pastoral"
            title={view.pastoral.title}
            description={view.pastoral.description}
          >
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
              {view.pastoral.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={8}
            variant="ogec"
            title={view.ogec.title}
            description={view.ogec.description}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 w-full">
              {view.ogec.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
          <OrganigramServiceFrame
            slotIndex={9}
            variant="tutelle"
            title={view.tutelle.title}
            description={view.tutelle.description}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 w-full">
              {view.tutelle.people.map((p) => (
                <PersonCard key={p.id} person={p} onSelect={setSelected} />
              ))}
            </div>
          </OrganigramServiceFrame>
        </div>
      </div>
      <div className="hidden print:block print:p-0">
        <OrganigramPrintDocument view={view} />
      </div>
      {selected ? (
        <div
          className="print:hidden fixed inset-0 z-[100] flex items-end sm:items-center justify-center p-4 bg-slate-900/50 backdrop-blur-sm"
          role="dialog"
          aria-modal="true"
          aria-labelledby="organigramme-modal-title"
        >
          <button
            type="button"
            className="absolute inset-0 cursor-default"
            aria-label="Fermer"
            onClick={() => setSelected(null)}
          />
          <div className="relative w-full max-w-lg rounded-2xl bg-white shadow-2xl border border-slate-200 overflow-hidden">
            <div className="flex items-start gap-4 p-6 border-b border-slate-100 bg-gradient-to-r from-slate-50 to-white">
              <div className="shrink-0 w-16 h-16 rounded-xl overflow-hidden bg-slate-200 flex items-center justify-center text-slate-700 font-bold text-lg">
                {selected.photoUrl ? (
                  <Image
                    src={selected.photoUrl}
                    alt=""
                    width={64}
                    height={64}
                    className="object-cover w-full h-full"
                  />
                ) : (
                  initials(selected)
                )}
              </div>
              <div className="min-w-0">
                <h3 id="organigramme-modal-title" className="text-lg font-bold text-slate-900">
                  {displayName(selected)}
                </h3>
                <p className="text-sky-800 font-medium text-sm mt-0.5">{selected.role}</p>
                {selected.email ? (
                  <a
                    href={`mailto:${selected.email}`}
                    className="text-xs text-sky-600 hover:underline mt-2 inline-block"
                  >
                    {selected.email}
                  </a>
                ) : null}
              </div>
              <button
                type="button"
                onClick={() => setSelected(null)}
                className="ml-auto shrink-0 rounded-lg p-2 text-slate-400 hover:bg-slate-100 hover:text-slate-700"
                aria-label="Fermer la fenêtre"
              >
                ✕
              </button>
            </div>
            <div className="p-6 max-h-[50vh] overflow-y-auto">
              <p className="text-xs font-semibold uppercase tracking-wide text-slate-500 mb-3">
                Missions & périmètre
              </p>
              <ul className="space-y-2">
                {modalMissions.length === 0 ? (
                  <li className="text-sm text-slate-500 italic">Aucune mission renseignée.</li>
                ) : (
                  modalMissions.map((line, i) => (
                    <li key={i} className="flex gap-2.5 text-sm text-slate-700 leading-relaxed">
                      <span className="shrink-0 text-slate-400 select-none" aria-hidden>
                        •
                      </span>
                      <span>{line}</span>
                    </li>
                  ))
                )}
              </ul>
            </div>
          </div>
        </div>
      ) : null}
      <style jsx global>{`
        @media print {
          @page {
            size: A4;
            margin: 5mm;
          }
          [aria-label*="IA" i],
          [aria-label*="assistant" i],
          [title*="IA" i],
          [title*="assistant" i],
          iframe[title*="assistant" i],
          iframe[title*="chat" i] {
            display: none !important;
            visibility: hidden !important;
          }
          html,
          body {
            background: white !important;
            print-color-adjust: exact;
            -webkit-print-color-adjust: exact;
          }
        }
      `}</style>
    </main>
  );
}

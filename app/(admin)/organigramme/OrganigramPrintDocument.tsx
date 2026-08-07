"use client";

import Image from "next/image";
import type { ReactNode } from "react";
import { useMemo } from "react";
import { SCHOOL } from "@/app/lib/school";
import type { OrganigramPerson } from "@/app/lib/organigramme";
import type { OrganigramView } from "@/app/lib/organigramme-resolve";

type PrintTone = | "direction" | "admin" | "accounting" | "reception" | "health" | "maintenance" | "poles" | "poleEcole" | "poleCollege" | "poleLycee" | "pastoral" | "ogec" | "tutelle";

const TILE: Record<
  PrintTone,
  { shell: string; heading: string; rule: string; missionBar: string }
> = {
  direction: {
    shell: "bg-indigo-50/80 border-indigo-100/90",
    heading: "text-indigo-900",
    rule: "border-indigo-200/70",
    missionBar: "border-indigo-200/50",
  },
  admin: {
    shell: "bg-sky-50/80 border-sky-100/90",
    heading: "text-sky-900",
    rule: "border-sky-200/70",
    missionBar: "border-sky-200/50",
  },
  accounting: {
    shell: "bg-emerald-50/80 border-emerald-100/90",
    heading: "text-emerald-900",
    rule: "border-emerald-200/70",
    missionBar: "border-emerald-200/50",
  },
  reception: {
    shell: "bg-violet-50/80 border-violet-100/90",
    heading: "text-violet-900",
    rule: "border-violet-200/70",
    missionBar: "border-violet-200/50",
  },
  health: {
    shell: "bg-teal-50/80 border-teal-100/90",
    heading: "text-teal-900",
    rule: "border-teal-200/70",
    missionBar: "border-teal-200/50",
  },
  maintenance: {
    shell: "bg-amber-50/80 border-amber-100/90",
    heading: "text-amber-900",
    rule: "border-amber-200/70",
    missionBar: "border-amber-200/50",
  },
  poles: {
    shell: "bg-cyan-50/75 border-cyan-100/90",
    heading: "text-cyan-900",
    rule: "border-cyan-200/70",
    missionBar: "border-cyan-200/45",
  },
  poleEcole: {
    shell: "bg-yellow-50/70 border-yellow-100/80",
    heading: "text-yellow-900",
    rule: "border-yellow-200/60",
    missionBar: "border-yellow-200/45",
  },
  poleCollege: {
    shell: "bg-blue-50/70 border-blue-100/80",
    heading: "text-blue-900",
    rule: "border-blue-200/60",
    missionBar: "border-blue-200/45",
  },
  poleLycee: {
    shell: "bg-fuchsia-50/65 border-fuchsia-100/80",
    heading: "text-fuchsia-900",
    rule: "border-fuchsia-200/60",
    missionBar: "border-fuchsia-200/45",
  },
  pastoral: {
    shell: "bg-rose-50/80 border-rose-100/90",
    heading: "text-rose-900",
    rule: "border-rose-200/70",
    missionBar: "border-rose-200/50",
  },
  ogec: {
    shell: "bg-slate-50/90 border-slate-200/90",
    heading: "text-slate-800",
    rule: "border-slate-300/80",
    missionBar: "border-slate-300/55",
  },
  tutelle: {
    shell: "bg-amber-50/85 border-amber-200/80",
    heading: "text-amber-950",
    rule: "border-amber-300/70",
    missionBar: "border-amber-300/50",
  },
};

function displayName(p: OrganigramPerson): string {
  const f = (p.firstName ?? "").trim();
  const l = (p.lastName ?? "").trim();
  if (f || l) return [f, l].filter(Boolean).join(" ");
  return "À compléter";
}

function initials(p: OrganigramPerson): string {
  const f = (p.firstName ?? "").trim();
  const l = (p.lastName ?? "").trim();
  if (f && l) return `${f[0]}${l[0]}`.toUpperCase();
  if (f) return f.slice(0, 2).toUpperCase();
  if (l) return l.slice(0, 2).toUpperCase();
  return "?";
}

function PrintPerson({ person, missionBarClass, className = "w-full"}: { person: OrganigramPerson; missionBarClass: string; className?: string}) {
  return (
    <div className={`print:break-inside-avoid rounded-2xl border border-slate-300 w-full bg-white/95 p-2 ${className}`}>
      <div className="flex items-start gap-1.5">
        <div className="w-10 h-10 shrink-0 rounded-xl overflow-hidden border border-slate-300 bg-slate-100 flex items-center justify-center text-[7pt] font-bold text-slate-600">
          {person.photoUrl ? (
            <Image src={person.photoUrl} alt="" width={40} height={40} quality={100} unoptimized className="w-full h-full object-cover" />
          ) : (
            initials(person)
          )}
        </div>
        <div className="min-w-0 flex-1">
          <p className="font-bold text-[7.5pt] leading-tight text-slate-800 m-0">
            {displayName(person)}
            <span className="font-semibold text-slate-700"> — {person.role}</span>
          </p>
          {person.email ? (
            <p className="text-[6.5pt] text-slate-600 m-0 mt-0.5 leading-tight">{person.email}</p>
          ) : null}
        </div>
      </div>
      <ul className={`list-none m-0 mt-0.5 pl-1.5 border-l ${missionBarClass} space-y-0`}>
        {person.missions.map((m, i) => (
          <li key={i} className="text-[6.5pt] leading-snug text-slate-700 pl-1 py-0">
            • {m.trim()}
          </li>
        ))}
      </ul>
    </div>
  );
}

function PrintTile({ tone, title, children}: { tone: PrintTone; title: string; description?: string; children: ReactNode}) {
  const t = TILE[tone];
  return (
    <section className={`print:break-inside-avoid rounded-2xl border p-4 mb-4 last:mb-0 ${t.shell}`}>
      <h2 className={`m-0 text-[7.5pt] font-bold uppercase tracking-wide pb-2 mb-3 border-b ${t.heading} ${t.rule}`}>
        {title}
      </h2>
      {children}
    </section>
  );
}

function PrintBlockTile({
  tone,
  title,
  description,
  people,
  personClassName = "w-full",
}: {
  tone: PrintTone;
  title: string;
  description?: string;
  people: OrganigramPerson[];
  personClassName?: string;
}) {
  const t = TILE[tone];
  const isOdd = people.length % 2 === 1;
  return (
    <PrintTile tone={tone} title={title} description={description}>
      <div className="grid grid-cols-2 gap-4">
        {people.map((p, index) => (
          <PrintPerson
            key={p.id}
            person={p}
            missionBarClass={t.missionBar}
            className={isOdd && index === people.length - 1 ? "col-span-2 w-full" : personClassName}
          />
        ))}
      </div>
    </PrintTile>
  );
}

function poleTone(id: string): PrintTone {
  if (id === "pole-ecole") return "poleEcole";
  if (id === "pole-college") return "poleCollege";
  return "poleLycee";
}

export function OrganigramPrintDocument({ view }: { view: OrganigramView }) {
  const printedAt = useMemo(
    () =>
      new Intl.DateTimeFormat("fr-FR", {
        day: "numeric",
        month: "long",
        year: "numeric",
      }).format(new Date()),
    [],
  );

  const dirT = TILE.direction;
  const halfWidthPerson = "w-full";

  return (
    <article className="organigram-print-root bg-white text-slate-800 max-w-[210mm] mx-auto print:max-w-none">
      <header className="rounded-2xl border border-slate-300 bg-slate-50 p-1.5 mb-1 print:break-inside-avoid">
        <h1 className="m-0 text-[11.5pt] font-black tracking-tight leading-tight text-slate-800">Organigramme interne</h1>
        <p className="m-0 mt-0.5 text-[8.5pt] font-semibold text-slate-700">{SCHOOL.shortName}</p>
        <p className="m-0 mt-0.5 text-[6.5pt] text-slate-600">{SCHOOL.address.fullCompact}</p>
        <p className="m-0 mt-1 text-[6pt] text-slate-500">Document du {printedAt} — usage interne</p>
      </header>

      <div className="flex flex-col text-[7pt] leading-[1.25]">
        <PrintTile tone="direction" title={view.sections.find(s => s.id === "direction")?.title || "Direction du groupe scolaire"} description={view.sections.find(s => s.id === "direction")?.description}>
          <div className="grid grid-cols-2 gap-4">
            {view.directors.map((p, index) => (
              <PrintPerson
                key={p.id}
                person={p}
                missionBarClass={dirT.missionBar}
                className={view.directors.length % 2 === 1 && index === view.directors.length - 1 ? "col-span-2 w-full" : halfWidthPerson}
              />
            ))}
          </div>
        </PrintTile>

        <PrintBlockTile
          tone="admin"
          title={view.admin.title}
          description={view.admin.description}
          people={view.admin.people}
          personClassName={halfWidthPerson}
        />
        <PrintBlockTile
          tone="accounting"
          title={view.accounting.title}
          description={view.accounting.description}
          people={view.accounting.people}
          personClassName={halfWidthPerson}
        />
        <PrintTile tone="poles" title={view.sections.find(s => s.id === "poles")?.title || "Pôles éducatifs & vie scolaire"} description={view.sections.find(s => s.id === "poles")?.description}>
          <div className="flex flex-col gap-4">
            {view.poles.map((pole) => {
              const pt = TILE[poleTone(pole.id)];
              return (
                <div key={pole.id} className={`rounded-2xl border p-4 print:break-inside-avoid ${pt.shell}`}>
                  <h3 className={`m-0 text-[7pt] font-bold pb-2 mb-3 border-b ${pt.heading} ${pt.rule}`}>{pole.label}</h3>
                  {pole.blocks.map((block) => (
                    <div key={block.id} className="mt-0.5 print:break-inside-avoid">
                        {(() => {
                        const isOdd = block.people.length % 2 === 1;
                        return (
                          <div className="grid grid-cols-2 gap-4">
                            {block.people.map((p, index) => (
                              <PrintPerson
                                key={p.id}
                                person={p}
                                missionBarClass={pt.missionBar}
                                className={isOdd && index === block.people.length - 1 ? "col-span-2 w-full" : halfWidthPerson}
                              />
                            ))}
                          </div>
                        );
                      })()}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        </PrintTile>
        <PrintBlockTile
          tone="reception"
          title={view.reception.title}
          description={view.reception.description}
          people={view.reception.people}
          personClassName={halfWidthPerson}
        />
        <PrintBlockTile
          tone="health"
          title={view.health.title}
          description={view.health.description}
          people={view.health.people}
          personClassName={halfWidthPerson}
        />
        <PrintBlockTile
          tone="maintenance"
          title={view.maintenance.title}
          description={view.maintenance.description}
          people={view.maintenance.people}
          personClassName={halfWidthPerson}
        />
        <PrintBlockTile
          tone="pastoral"
          title={view.pastoral.title}
          description={view.pastoral.description}
          people={view.pastoral.people}
          personClassName={halfWidthPerson}
        />
        <PrintBlockTile
          tone="ogec"
          title={view.ogec.title}
          description={view.ogec.description}
          people={view.ogec.people}
          personClassName={halfWidthPerson}
        />
        <PrintBlockTile
          tone="tutelle"
          title={view.tutelle.title}
          description={view.tutelle.description}
          people={view.tutelle.people}
          personClassName={halfWidthPerson}
        />
      </div>
    </article>
  );
}

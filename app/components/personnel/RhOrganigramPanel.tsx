"use client";

import Link from "next/link";
import {
  PERSONNEL_CATEGORY_LABELS,
  type PersonnelCategory,
  type PersonnelIndexEntry,
} from "@/app/lib/personnel-types";

const ORDER: PersonnelCategory[] = ["cpe", "education", "administratif", "comptabilite", "maintenance"];

export default function RhOrganigramPanel({ index }: { index: PersonnelIndexEntry[] }) {
  const active = index.filter((p) => p.active);

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/50 p-4 text-sm text-indigo-950">
        Vue par pôle — annuaire RH synchronisé avec les fiches collaborateurs.
      </div>
      {ORDER.map((cat) => {
        const group = active.filter((p) => p.category === cat);
        if (!group.length) return null;
        return (
          <section key={cat} className="bg-white rounded-2xl border border-slate-200 p-5">
            <h3 className="font-black text-slate-900 mb-3">{PERSONNEL_CATEGORY_LABELS[cat]}</h3>
            <ul className="grid sm:grid-cols-2 lg:grid-cols-3 gap-2">
              {group.map((p) => (
                <li key={p.id}>
                  <Link
                    href={`/rh/${p.id}`}
                    className="block rounded-xl border border-slate-100 px-3 py-2 hover:border-indigo-200 hover:bg-indigo-50/40 transition"
                  >
                    <p className="font-bold text-sm text-slate-900">{p.displayName}</p>
                    <p className="text-xs text-slate-500">{p.email}</p>
                  </Link>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </div>
  );
}

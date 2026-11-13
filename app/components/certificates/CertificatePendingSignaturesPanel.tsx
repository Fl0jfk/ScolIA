"use client";

import Link from "@/app/components/ui/AppLink";
import { useEffect, useState } from "react";
import { STUDENT_AWARD_STATUS_LABELS } from "@/app/lib/certificates-types";

type Pending = {
  prof: Array<{ awardId: string; programId: string; programTitle: string; studentName: string }>;
  direction: Array<{ awardId: string; programId: string; programTitle: string; studentName: string; secteur: string }>;
};

export default function CertificatePendingSignaturesPanel() {
  const [pending, setPending] = useState<Pending | null>(null);

  useEffect(() => {
    fetch("/api/certificates/pending-signatures", { cache: "no-store" })
      .then((r) => r.json())
      .then(setPending)
      .catch(() => setPending({ prof: [], direction: [] }));
  }, []);

  if (!pending) return null;
  if (!pending.prof.length && !pending.direction.length) return null;

  return (
    <div className="mx-4 md:mx-0 mb-6 rounded-2xl border border-amber-300 bg-amber-50 p-4 space-y-3">
      <p className="text-sm font-black text-amber-900">Signatures en attente</p>
      {pending.prof.length > 0 && (
        <div>
          <p className="text-xs font-bold text-amber-800 mb-2">En tant qu&apos;enseignant</p>
          <ul className="space-y-1">
            {pending.prof.map((p) => (
              <li key={p.awardId}>
                <Link
                  href={`/certificates/${p.programId}/${p.awardId}`}
                  className="text-sm text-amber-900 underline font-medium"
                >
                  {p.studentName} — {p.programTitle}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      )}
      {pending.direction.length > 0 && (
        <div>
          <p className="text-xs font-bold text-amber-800 mb-2">En tant que direction</p>
          <ul className="space-y-1">
            {pending.direction.map((p) => (
              <li key={p.awardId}>
                <Link
                  href={`/certificates/${p.programId}/${p.awardId}`}
                  className="text-sm text-amber-900 underline font-medium"
                >
                  {p.studentName} — {p.programTitle} ({p.secteur})
                </Link>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export function AwardStatusBadge({ status }: { status: string }) {
  const label = STUDENT_AWARD_STATUS_LABELS[status as keyof typeof STUDENT_AWARD_STATUS_LABELS] || status;
  const colors: Record<string, string> = {
    draft: "bg-slate-100 text-slate-700",
    submitted: "bg-blue-100 text-blue-800",
    prof_signed: "bg-violet-100 text-violet-800",
    direction_signed: "bg-amber-100 text-amber-900",
    issued: "bg-emerald-100 text-emerald-800",
  };
  return (
    <span className={`inline-flex rounded-lg px-2 py-0.5 text-xs font-black ${colors[status] || colors.draft}`}>
      {label}
    </span>
  );
}

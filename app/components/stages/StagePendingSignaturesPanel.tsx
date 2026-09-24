"use client";

import type { PendingStageSignature } from "@/app/lib/stage-pending-signatures";
import StageOutOfPeriodAlert from "@/app/components/stages/StageOutOfPeriodAlert";

export default function StagePendingSignaturesPanel({
  items,
  hasStoredSignature,
}: {
  items: PendingStageSignature[];
  hasStoredSignature?: boolean;
}) {
  if (!items.length) return null;

  const hasOutside = items.some((i) => i.outsideOfficialPeriod);

  return (
    <section className="mb-6 rounded-2xl border-2 border-amber-300 bg-amber-50 p-5 shadow-sm">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-black text-amber-950">
            {items.length} convention{items.length > 1 ? "s" : ""} à signer
          </h2>
          <p className="mt-1 text-sm text-amber-900 max-w-2xl">
            L&apos;administration a validé {items.length > 1 ? "ces dossiers" : "ce dossier"} — votre
            paraphe doit être apposé sur le PDF. Un e-mail vous a aussi été envoyé avec le lien direct.
          </p>
          {hasStoredSignature === false && (
            <p className="mt-2 text-xs font-semibold text-violet-900">
              Astuce : enregistrez d&apos;abord votre signature dans Mon compte → Sécurité → Ma
              signature pour signer en un clic.
            </p>
          )}
        </div>
        <span className="rounded-full bg-amber-400 px-3 py-1 text-xs font-black text-amber-950">
          Action requise
        </span>
      </div>

      {hasOutside ? (
        <div className="mt-4">
          <StageOutOfPeriodAlert
            alignment={{
              status: "outside",
              outside: true,
              className: "",
              scheduleStart: "",
              scheduleEnd: "",
              scheduleLabel: "voir détail ci-dessous",
              officialPeriods: [],
              referencePeriod: null,
              daysFullyOutside: 0,
              daysOutsideReference: 0,
              message:
                "Au moins une convention à signer est hors des périodes officielles de stage. Vérifiez les dates avant de parapher.",
              shortMessage: "HORS PÉRIODE OFFICIELLE",
            }}
          />
        </div>
      ) : null}

      <ul className="mt-4 space-y-3">
        {items.map((item) => (
          <li
            key={`${item.conventionId}-${item.signatureId}`}
            className={`flex flex-wrap items-center justify-between gap-3 rounded-xl border px-4 py-3 ${
              item.outsideOfficialPeriod
                ? "border-4 border-rose-600 bg-rose-50"
                : "border-amber-200 bg-white"
            }`}
          >
            <div className="text-sm">
              <p className="font-bold text-[#1F3D2B]">
                {item.studentName}
                <span className="font-normal text-stone-500"> — {item.className}</span>
              </p>
              <p className="text-stone-600">
                {item.companyName} · {item.periodStart} → {item.periodEnd}
              </p>
              <p className="text-xs text-stone-500 mt-0.5">En tant que {item.roleLabel}</p>
              {item.outsideOfficialPeriod ? (
                <StageOutOfPeriodAlert
                  compact
                  alignment={{
                    status: "outside",
                    outside: true,
                    className: item.className,
                    scheduleStart: item.periodStart,
                    scheduleEnd: item.periodEnd,
                    scheduleLabel: `${item.periodStart} → ${item.periodEnd}`,
                    officialPeriods: [],
                    referencePeriod: null,
                    daysFullyOutside: 0,
                    daysOutsideReference: 0,
                    message: item.outsideOfficialPeriodMessage || "Hors période officielle",
                    shortMessage: item.outsideOfficialPeriodMessage || "HORS PÉRIODE OFFICIELLE",
                  }}
                />
              ) : null}
            </div>
            <a
              href={item.signLink}
              className={`shrink-0 rounded-lg px-4 py-2 text-sm font-bold text-white hover:opacity-90 ${
                item.outsideOfficialPeriod ? "bg-rose-700" : "bg-[#2F6B4A] hover:bg-[#255a3d]"
              }`}
            >
              {item.outsideOfficialPeriod ? "Vérifier puis signer" : "Signer maintenant"}
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}

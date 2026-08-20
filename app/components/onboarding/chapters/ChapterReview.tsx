"use client";

import type {
  Establishment,
  ExternalQuickLinkConfig,
  SiteIdentity,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import { dash } from "@/app/lib/dashboard-brand";

type Props = {
  identity: Partial<SiteIdentity>;
  establishments: Establishment[];
  travels: Partial<TravelsModuleConfig>;
  wantQuickLinks: boolean;
  externalLinks: ExternalQuickLinkConfig[];
};

export default function ChapterReview({
  identity,
  establishments,
  travels,
  wantQuickLinks,
  externalLinks,
}: Props) {
  const rows = [
    {
      label: "Organisation",
      value: `${identity.name || "—"} (${
        identity.organizationKind === "groupe" ? "groupe scolaire" : "établissement unique"
      })`,
    },
    {
      label: "Identité",
      value: [
        identity.shortName || identity.name,
        identity.dashboardAccent ? `accent ${identity.dashboardAccent}` : null,
      ]
        .filter(Boolean)
        .join(" · "),
    },
    {
      label: "Adresse",
      value: [identity.address?.street, identity.address?.zip, identity.address?.city]
        .filter(Boolean)
        .join(", ") || "—",
    },
    {
      label: "Établissements",
      value: establishments.map((e) => e.label).join(", ") || "—",
    },
    {
      label: "Directions",
      value:
        establishments
          .map((e) => (e.directorName || e.directorEmail ? `${e.label}: ${e.directorName || e.directorEmail}` : null))
          .filter(Boolean)
          .join(" · ") || "À compléter plus tard",
    },
    {
      label: "Transporteurs",
      value: String((travels.transportProviders || []).filter((p) => p.name || p.email).length),
    },
    {
      label: "Raccourcis",
      value: String(wantQuickLinks ? externalLinks.filter((l) => l.name && l.link).length : 0),
    },
  ];

  return (
    <div>
      <p className={`mb-5 text-sm leading-relaxed ${dash.textMid}`}>
        Vérifiez les informations. Vous pourrez tout modifier ensuite dans Paramètres généraux.
        L&apos;étape suivante concerne les licences Microsoft (A3 / A1).
      </p>
      <ul className="space-y-3">
        {rows.map((row) => (
          <li
            key={row.label}
            className="flex flex-col gap-0.5 rounded-2xl border border-white/70 bg-white/55 px-4 py-3 sm:flex-row sm:items-baseline sm:justify-between sm:gap-4"
          >
            <span className={`text-[10px] font-bold uppercase tracking-[0.18em] ${dash.label}`}>
              {row.label}
            </span>
            <span className={`text-sm font-medium sm:text-right ${dash.ink}`}>{row.value}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

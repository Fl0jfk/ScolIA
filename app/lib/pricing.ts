/**

 * Grille tarifaire Scola — logique partagée site + futur Stripe Billing.

 * Stripe : activer quand la société et le compte Stripe seront créés (voir STRIPE_BILLING).

 */

import { MARKETING } from "@/app/lib/marketing-site";

/** Tarif de référence : €/élève/mois */

const PRICE_PER_STUDENT_MONTHLY_EUR = 0.3;

/** Réduction paiement annuel (à la rentrée) */

const ANNUAL_UPFRONT_DISCOUNT = 0.1;
export const INCLUDED_A3_LICENSES = 10;
export const EXTRA_A3_PRICE_MONTHLY_EUR = 5;

export type BillingMode = "monthly" | "annual_upfront";

type PricingBreakdown = {

  studentCount: number;

  mode: BillingMode;

  pricePerStudentMonth: number;

  pricePerStudentYear: number;

  monthlyTotal: number;

  annualTotal: number;

  discountPercent: number;
  includedA3: number;
  extraA3Count: number;
  extraA3UnitMonthly: number;
  extraA3MonthlyTotal: number;
  extraA3AnnualTotal: number;
  totalMonthlyWithExtras: number;
  totalAnnualWithExtras: number;

};

function clampStudentCount(value: number): number {

  if (!Number.isFinite(value) || value < 0) return 0;

  return Math.round(value);

}

function computePricing(studentCount: number, mode: BillingMode): PricingBreakdown {

  const students = clampStudentCount(studentCount);

  const baseMonthly = PRICE_PER_STUDENT_MONTHLY_EUR;

  const annualFull = baseMonthly * 12 * students;

  const extrasCount = 0;
  const extraMonthly = extrasCount * EXTRA_A3_PRICE_MONTHLY_EUR;
  const extraAnnual = extraMonthly * 12;

  if (mode === "monthly") {

    return {

      studentCount: students,

      mode,

      pricePerStudentMonth: baseMonthly,

      pricePerStudentYear: baseMonthly * 12,

      monthlyTotal: baseMonthly * students,

      annualTotal: annualFull,

      discountPercent: 0,
      includedA3: INCLUDED_A3_LICENSES,
      extraA3Count: extrasCount,
      extraA3UnitMonthly: EXTRA_A3_PRICE_MONTHLY_EUR,
      extraA3MonthlyTotal: extraMonthly,
      extraA3AnnualTotal: extraAnnual,
      totalMonthlyWithExtras: baseMonthly * students + extraMonthly,
      totalAnnualWithExtras: annualFull + extraAnnual,

    };

  }

  const annualDiscounted = annualFull * (1 - ANNUAL_UPFRONT_DISCOUNT);

  return {

    studentCount: students,

    mode,

    pricePerStudentMonth: annualDiscounted / (12 * Math.max(students, 1)),

    pricePerStudentYear: annualDiscounted / Math.max(students, 1),

    monthlyTotal: annualDiscounted / 12,

    annualTotal: annualDiscounted,

    discountPercent: Math.round(ANNUAL_UPFRONT_DISCOUNT * 100),
    includedA3: INCLUDED_A3_LICENSES,
    extraA3Count: extrasCount,
    extraA3UnitMonthly: EXTRA_A3_PRICE_MONTHLY_EUR,
    extraA3MonthlyTotal: extraMonthly,
    extraA3AnnualTotal: extraAnnual,
    totalMonthlyWithExtras: annualDiscounted / 12 + extraMonthly,
    totalAnnualWithExtras: annualDiscounted + extraAnnual,

  };

}

export function normalizeExtraA3Count(value: number): number {
  if (!Number.isFinite(value) || value < 0) return 0;
  return Math.round(value);
}

export function computePricingWithA3Extras(
  studentCount: number,
  mode: BillingMode,
  extraA3Count: number,
): PricingBreakdown {
  const base = computePricing(studentCount, mode);
  const extras = normalizeExtraA3Count(extraA3Count);
  const extraMonthly = extras * EXTRA_A3_PRICE_MONTHLY_EUR;
  const extraAnnual = extraMonthly * 12;
  return {
    ...base,
    extraA3Count: extras,
    extraA3MonthlyTotal: extraMonthly,
    extraA3AnnualTotal: extraAnnual,
    totalMonthlyWithExtras: base.monthlyTotal + extraMonthly,
    totalAnnualWithExtras: base.annualTotal + extraAnnual,
  };
}

export function formatEur(amount: number, opts?: { decimals?: number }): string {

  const decimals = opts?.decimals ?? (amount < 10 ? 2 : 0);

  return new Intl.NumberFormat("fr-FR", {

    style: "currency",

    currency: "EUR",

    minimumFractionDigits: decimals,

    maximumFractionDigits: decimals,

  }).format(amount);

}

export const BILLING_OPTIONS = [

  {

    id: "monthly" as const,

    mode: "monthly" as BillingMode,

    name: "Mensuel",

    badge: "Sans engagement",

    highlighted: true,

    priceLine: "Sur devis",

    summary: "Comme un abonnement classique : vous payez au mois, vous résiliez quand vous voulez.",

    perks: [

      "Résiliation à tout moment",

      "Facturation au mois selon l'effectif",

      "Idéal pour tester ou démarrer en cours d'année",

    ],

  },

  {

    id: "annual_upfront" as const,

    mode: "annual_upfront" as BillingMode,

    name: "Annuel",

    badge: "−10 %",

    highlighted: false,

    priceLine: "Sur devis (−10 %)",

    summary: "Un seul paiement à la rentrée — budget annuel simplifié pour l'OGEC.",

    perks: [

      "10 % de réduction sur l'année",

      "Paiement unique (virement ou CB à l'activation)",

      "Budget annuel simplifié pour l'OGEC",

    ],

  },

] as const;

/**

 * Préparation Stripe — à brancher après création de la société.

 *

 * Variables d'environnement prévues :

 * - NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY

 * - STRIPE_SECRET_KEY (serveur uniquement)

 * - STRIPE_PRICE_ID_MONTHLY_PER_STUDENT (metered or per-unit quantity)

 * - STRIPE_PRICE_ID_ANNUAL_PER_STUDENT

 * - STRIPE_WEBHOOK_SECRET

 */

export const STRIPE_BILLING = {

  isConfigured: Boolean(process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY?.trim()),

  publishableKey: process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY?.trim() ?? "",

  priceIds: {

    monthlyPerStudent: process.env.STRIPE_PRICE_ID_MONTHLY_PER_STUDENT?.trim() ?? "",

    annualPerStudent: process.env.STRIPE_PRICE_ID_ANNUAL_PER_STUDENT?.trim() ?? "",

  },

} as const;

/** Paiement abonnement via Easytransac (Open Banking + prélèvement). */
const EASYTRANSAC_BILLING = {
  isConfigured: Boolean(process.env.EASYTRANSAC_API_KEY?.trim()),
} as const;

function getSubscribeCta(mode: BillingMode): {
  label: string;
  href: string;
  stripeReady: boolean;
  easytransacReady: boolean;
} {
  const stripeReady = STRIPE_BILLING.isConfigured;
  const easytransacReady = EASYTRANSAC_BILLING.isConfigured;

  if (easytransacReady) {
    return {
      label: "Déposer un dossier",
      href: "/souscrire",
      stripeReady: false,
      easytransacReady: true,
    };
  }

  if (stripeReady) {

    return {

      label: "S'abonner en ligne",

      href: `/api/billing/checkout?mode=${mode}`,

      stripeReady: true,
      easytransacReady: false,
    };
  }

  return {
    label: "Nous contacter pour souscrire",
    href: `mailto:${MARKETING.contactEmail}?subject=${encodeURIComponent(`Abonnement ${MARKETING.productName} — ${BILLING_OPTIONS.find((o) => o.mode === mode)?.name ?? mode}`)}`,
    stripeReady: false,
    easytransacReady: false,
  };
}


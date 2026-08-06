import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  canAccessHseModule,
  canCreateHseDemand,
  canViewHseDemand,
  type HseEtablissement,
} from "@/app/lib/demandes-hse-access";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import { wizardStep } from "@/app/lib/brain-ai/wizard";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

const INDEX_KEY = "demandes-hse/index.json";

type HseRecord = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "ANNULEE";
  createdBy: { userId: string; name: string; email: string };
  etablissement: HseEtablissement;
  resumeDemande: string;
  motif: string;
  nombreHeures?: number;
  classe: string;
  details: string;
};

function isValidEtab(v: string): v is HseEtablissement {
  return v === "École" || v === "Collège" || v === "Lycée";
}

function parseNombreHeures(raw: unknown): { ok: true; value: number } | { ok: false; error: string } {
  const n =
    typeof raw === "number" ? raw : Number(String(raw ?? "").trim().replace(",", "."));
  if (!Number.isFinite(n) || n <= 0) {
    return { ok: false, error: "Indiquez le nombre d'heures demandé (supérieur à 0)." };
  }
  const quarters = Math.round(n * 4);
  if (Math.abs(n * 4 - quarters) > 1e-6) {
    return { ok: false, error: "Le nombre d'heures doit être un multiple de 0,25." };
  }
  if (quarters > 4000) return { ok: false, error: "Nombre d'heures trop élevé." };
  return { ok: true, value: quarters / 4 };
}

function formatNombreHeures(h: number): string {
  const text = Number.isInteger(h) ? String(h) : h.toFixed(2).replace(/\.?0+$/, "").replace(".", ",");
  return `${text} h`;
}

async function getIndex(): Promise<HseRecord[]> {
  const hit = await getJson<HseRecord[]>(INDEX_KEY);
  return hit?.data ?? [];
}

export async function handleListHseDemands(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  if (!canAccessHseModule(ctx.roles)) {
    return { ok: false, error: "Accès HSE réservé.", code: "MODULE_FORBIDDEN" };
  }

  const statusFilter = typeof args.status === "string" ? args.status.trim().toUpperCase() : "";
  const limit = Math.min(Math.max(Number(args.limit) || 15, 1), 40);
  const all = await getIndex();
  let items = all
    .filter((r) => canViewHseDemand(r, ctx.userId!, ctx.roles))
    .sort((a, b) => +new Date(b.createdAt) - +new Date(a.createdAt));
  if (statusFilter) items = items.filter((r) => r.status === statusFilter);

  const brief = items.slice(0, limit).map((r) => ({
    id: r.id,
    status: r.status,
    etablissement: r.etablissement,
    nombreHeures: r.nombreHeures ?? null,
    resume: r.resumeDemande.slice(0, 120),
    classe: r.classe,
    createdAt: r.createdAt.slice(0, 10),
    mine: r.createdBy.userId === ctx.userId,
  }));

  return {
    ok: true,
    data: {
      items: brief,
      totalVisible: items.length,
      ctas: [{ label: "Ouvrir HSE", href: "/rh?tab=hse" }],
    },
    summaryFr:
      brief.length === 0
        ? "Aucune demande HSE visible."
        : `${brief.length} demande(s) HSE : ${brief
            .slice(0, 5)
            .map((i) => `${i.status}${i.nombreHeures != null ? ` ${formatNombreHeures(i.nombreHeures)}` : ""}`)
            .join(" · ")}.`,
  };
}

export async function handleCreateHseDemand(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  if (!canCreateHseDemand(ctx.roles)) {
    return {
      ok: false,
      error: "Seuls les enseignants peuvent créer une demande HSE.",
      code: "MODULE_FORBIDDEN",
    };
  }
  if (!ctx.email) {
    return { ok: false, error: "Votre compte doit avoir une adresse e-mail." };
  }

  let etablissement = String(args.etablissement || "").trim();
  let resumeDemande = String(args.resumeDemande || "").trim();
  let classe = String(args.classe || "").trim();
  let details = String(args.details || "").trim();
  const detailsResolved = Boolean(args.detailsResolved);
  const heuresParsed = args.nombreHeures != null && String(args.nombreHeures).trim() !== ""
    ? parseNombreHeures(args.nombreHeures)
    : null;

  const total = 4;
  let step = 1;
  const draft = (): Record<string, unknown> => ({
    etablissement,
    resumeDemande,
    classe,
    details,
    ...(heuresParsed?.ok ? { nombreHeures: heuresParsed.value } : {}),
    ...(detailsResolved ? { detailsResolved: true } : {}),
  });

  if (!isValidEtab(etablissement)) {
    return choicesResult(
      "create_hse_demand",
      "etablissement",
      wizardStep(step, total, "Demande HSE — pour quel établissement ?"),
      [
        { value: "École", label: "École" },
        { value: "Collège", label: "Collège" },
        { value: "Lycée", label: "Lycée" },
      ],
      draft(),
    );
  }
  step += 1;

  if (!resumeDemande) {
    return choicesResult(
      "create_hse_demand",
      "resumeDemande",
      wizardStep(step, total, "Décrivez brièvement votre demande HSE :"),
      [],
      draft(),
      "text",
    );
  }
  step += 1;

  if (!heuresParsed || !heuresParsed.ok) {
    return choicesResult(
      "create_hse_demand",
      "nombreHeures",
      wizardStep(
        step,
        total,
        heuresParsed && !heuresParsed.ok
          ? heuresParsed.error
          : "Combien d'heures ? (ex. 2 ou 1,5 — multiple de 0,25)",
      ),
      [],
      draft(),
      "text",
    );
  }
  step += 1;

  if (!classe) {
    return choicesResult(
      "create_hse_demand",
      "classe",
      wizardStep(step, total, "Classe ou contexte pédagogique ?"),
      [],
      draft(),
      "text",
    );
  }

  if (!detailsResolved && !details) {
    return choicesResult(
      "create_hse_demand",
      "details",
      wizardStep(4, 4, "Précisions complémentaires ?"),
      [
        { value: "Non", label: "Non" },
        { value: "__CUSTOM__", label: "Oui, saisir…" },
      ],
      draft(),
    );
  }
  if (details === "__CUSTOM__") {
    const custom = String(args.detailsCustom || "").trim();
    if (!custom) {
      return choicesResult(
        "create_hse_demand",
        "detailsCustom",
        wizardStep(4, 4, "Vos précisions :"),
        [],
        { ...draft(), details: "__CUSTOM__" },
        "text",
      );
    }
    details = custom;
  } else if (details === "Non") {
    details = "";
  }

  const nombreHeures = heuresParsed.value;

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_hse_demand",
      args: {
        etablissement,
        resumeDemande,
        nombreHeures,
        classe,
        details,
        detailsResolved: true,
      },
      summaryFr:
        `Récapitulatif — Demande HSE\n` +
        `• Établissement : ${etablissement}\n` +
        `• Heures : ${formatNombreHeures(nombreHeures)}\n` +
        `• Classe : ${classe}\n` +
        `• Demande : ${resumeDemande.slice(0, 160)}${resumeDemande.length > 160 ? "…" : ""}` +
        (details ? `\n• Précisions : ${details.slice(0, 120)}${details.length > 120 ? "…" : ""}` : ""),
    };
  }

  const record: HseRecord = {
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    status: "EN_ATTENTE",
    createdBy: {
      userId: ctx.userId,
      name: [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || ctx.email,
      email: ctx.email,
    },
    etablissement,
    resumeDemande,
    motif: resumeDemande,
    nombreHeures,
    classe,
    details,
  };

  const all = await getIndex();
  all.push(record);
  await putJson(INDEX_KEY, all);

  try {
    const bundle = await loadAppConfig();
    const est = getEstablishmentByLabel(bundle, etablissement);
    const dirEmail = est?.directorEmail || "";
    const dirName = est?.directorName || est?.label || etablissement;
    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (transporter && smtp && dirEmail) {
      const link = await tenantAbsolutePath("/demandes-hse");
      await transporter.sendMail({
        from: `"Demandes HSE" <${smtp.user}>`,
        to: dirEmail,
        subject: `HSE — nouvelle demande (${etablissement})`,
        text: [
          `Bonjour ${dirName},`,
          ``,
          `Demandeur : ${record.createdBy.name} (${record.createdBy.email})`,
          `Établissement : ${etablissement}`,
          `Heures : ${formatNombreHeures(nombreHeures)}`,
          `Demande : ${resumeDemande}`,
          `Classe : ${classe}`,
          details ? `Précisions : ${details}` : "",
          ``,
          `Traiter : ${link}`,
        ]
          .filter(Boolean)
          .join("\n"),
      });
    }
  } catch (err) {
    console.warn("[brain-ai] hse mail failed", err);
  }

  return {
    ok: true,
    data: {
      id: record.id,
      followUrl: "/rh?tab=hse",
      ctas: [{ label: "Suivre dans RH / HSE", href: "/rh?tab=hse" }],
    },
    summaryFr: `Demande HSE créée (${record.id}) — ${formatNombreHeures(nombreHeures)}.`,
  };
}

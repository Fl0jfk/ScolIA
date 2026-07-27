import "server-only";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { loadAllTenants } from "@/app/lib/tenant-registry";

export type TenantMatchCandidate = {
  slug: string;
  label: string;
  street: string | null;
  zip: string | null;
  city: string | null;
};

/** Tenants établissement (hors plateforme) pour le matching IA. */
export async function listTenantMatchCandidates(): Promise<TenantMatchCandidate[]> {
  const tenants = await loadAllTenants();
  return tenants
    .filter((t) => !isPlatformTenantSlug(t.slug))
    .map((t) => ({
      slug: t.slug,
      label: t.label || t.slug,
      street: t.postalAddress?.street?.trim() || null,
      zip: t.postalAddress?.zip?.trim() || null,
      city: t.postalAddress?.city?.trim() || null,
    }));
}

export type TenantMatchResult = {
  slug: string | null;
  motif: string | null;
};

/**
 * Déduit le tenant à partir du contenu mail (± OCR devis) quand le tag Reply-To
 * `mailer+{slug}@…` est absent.
 */
export async function resolveTenantSlugWithMistral(input: {
  subject?: string;
  bodyPlain?: string;
  snippet?: string;
  ocrText?: string;
  fromEmail?: string;
  candidates?: TenantMatchCandidate[];
}): Promise<TenantMatchResult> {
  const candidates = input.candidates ?? (await listTenantMatchCandidates());
  if (candidates.length === 0) {
    return { slug: null, motif: "aucun_tenant_en_catalogue" };
  }
  if (candidates.length === 1) {
    return { slug: candidates[0]!.slug, motif: "tenant_unique_catalogue" };
  }

  const mistralKey = await getMistralApiKey();
  if (!mistralKey) {
    return { slug: null, motif: "missing_mistral_key" };
  }

  const catalogue = candidates
    .map((c, i) => {
      const addr = [c.street, c.zip, c.city].filter(Boolean).join(", ") || "adresse non renseignée";
      return `${i + 1}. slug="${c.slug}" | nom="${c.label}" | adresse="${addr}"`;
    })
    .join("\n");

  const mailBlock = [
    input.fromEmail ? `De: ${input.fromEmail}` : "",
    input.subject ? `Objet: ${input.subject}` : "",
    input.bodyPlain || input.snippet
      ? `Corps:\n${(input.bodyPlain || input.snippet || "").slice(0, 6000)}`
      : "",
    input.ocrText ? `OCR devis PDF:\n${input.ocrText.slice(0, 8000)}` : "",
  ]
    .filter(Boolean)
    .join("\n\n");

  if (mailBlock.trim().length < 12) {
    return { slug: null, motif: "contenu_insuffisant_tenant" };
  }

  const prompt = `Tu analyses un e-mail (éventuellement un devis transporteur scolaire) destiné à la plateforme ScolIA.
Le destinataire n'a PAS le tag tenant dans l'adresse (pas de mailer+slug@…).
Tu dois identifier QUEL établissement / groupe scolaire (tenant) est concerné.

Indices typiques dans le mail ou le devis : nom d'école, collège, lycée, groupe scolaire, adresse, code postal, ville, raison sociale.

Catalogue des tenants possibles (tu DOIS répondre avec l'un de ces slug, ou null) :
${catalogue}

Message à analyser :
---
${mailBlock}
---

Règles :
- Réponds UNIQUEMENT en JSON : {"slug":"<slug du catalogue ou null>","motif":"courte justification"}
- Si plusieurs candidats possibles ou doute → slug null
- N'invente jamais un slug hors catalogue
- Ne choisis pas au hasard`;

  try {
    const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${mistralKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0,
        response_format: { type: "json_object" },
        messages: [{ role: "user", content: prompt }],
      }),
    });
    if (!res.ok) {
      return { slug: null, motif: "erreur_http_mistral_tenant" };
    }
    const data = (await res.json()) as {
      choices?: Array<{ message?: { content?: string } }>;
    };
    const raw = data.choices?.[0]?.message?.content?.trim() || "";
    if (!raw) return { slug: null, motif: "reponse_mistral_tenant_vide" };

    const start = raw.indexOf("{");
    const end = raw.lastIndexOf("}");
    if (start < 0 || end < start) return { slug: null, motif: "json_tenant_invalide" };
    const parsed = JSON.parse(raw.slice(start, end + 1)) as {
      slug?: string | null;
      motif?: string | null;
    };

    const wanted = typeof parsed.slug === "string" ? parsed.slug.trim().toLowerCase() : "";
    const allowed = new Set(candidates.map((c) => c.slug.toLowerCase()));
    if (!wanted || !allowed.has(wanted)) {
      return {
        slug: null,
        motif: parsed.motif ? String(parsed.motif).slice(0, 400) : "tenant_introuvable_ia",
      };
    }

    const hit = candidates.find((c) => c.slug.toLowerCase() === wanted);
    return {
      slug: hit?.slug ?? wanted,
      motif: parsed.motif ? String(parsed.motif).slice(0, 400) : "match_tenant_ia",
    };
  } catch (e) {
    console.error("[travel-tenant-match]", e);
    return { slug: null, motif: "erreur_mistral_tenant" };
  }
}

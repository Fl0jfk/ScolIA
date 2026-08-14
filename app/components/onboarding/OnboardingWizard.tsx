"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { DASHBOARD_ACCENT_OPTIONS } from "@/app/lib/dashboard-brand-presets";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import type {
  Establishment,
  EstablishmentKind,
  ExternalQuickLinkConfig,
  IntegrationsConfig,
  NotificationsConfig,
  SiteIdentity,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import { ESTABLISHMENT_KIND_PRESETS } from "@/app/lib/establishment-visual";
import { clerkRoleSlugsForEstablishment, directionRoleForKind } from "@/app/lib/establishment-catalog";
import ClerkPersonSelect from "@/app/components/settings/ClerkPersonSelect";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

const TOTAL_STEPS = 12;

import { newQuickLinkSlot } from "@/app/lib/dashboard-quick-links";

function normalizeOnboardingStep(saved: number): number {
  if (!Number.isFinite(saved) || saved < 1) return 1;
  if (saved <= 4) return saved;
  if (saved === 5) return 5;
  if (saved >= 13) return 12;
  return Math.min(saved - 1, 12);
}

type EstablishmentDraft = Establishment;

function memberDisplayName(m: ClerkMemberOption): string {
  return m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email;
}

function Help({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-sm text-stone-600 leading-relaxed mb-4 bg-[#F7F4EF] border border-stone-200 rounded-xl p-4">
      {children}
    </p>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block mb-4">
      <span className="block text-sm font-medium text-stone-700 mb-1">{label}</span>
      {children}
    </label>
  );
}

const inputClass =
  "w-full rounded-xl border border-stone-200 bg-white px-3 py-2.5 text-sm outline-none focus:border-[#2F6B4A] focus:ring-2 focus:ring-emerald-100";

export default function OnboardingWizard() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const reviewMode = searchParams.get("review") === "1";

  const [step, setStep] = useState(1);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [identity, setIdentity] = useState<Partial<SiteIdentity>>({});
  const [establishments, setEstablishments] = useState<EstablishmentDraft[]>([]);
  const [notifications, setNotifications] = useState<Partial<NotificationsConfig>>({});
  const [travels, setTravels] = useState<Partial<TravelsModuleConfig>>({ transportProviders: [] });
  const [integrations, setIntegrations] = useState<IntegrationsConfig>({});
  const [externalLinks, setExternalLinks] = useState<ExternalQuickLinkConfig[]>([]);
  const [wantQuickLinks, setWantQuickLinks] = useState(false);
  const [hasInternat, setHasInternat] = useState(false);
  const [existingConfigDetected, setExistingConfigDetected] = useState(false);
  const [clerkMembers, setClerkMembers] = useState<ClerkMemberOption[]>([]);
  const [membersLoading, setMembersLoading] = useState(false);

  const loadConfig = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings");
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      const cfg = j.config;
      const loadedEstablishments = (cfg.establishments || []) as Establishment[];
      const identityCfg = cfg.identity || {};
      const activeCount = loadedEstablishments.filter((e: EstablishmentDraft) => e.active !== false).length;
      const hasRealName = Boolean(identityCfg.name?.trim() && identityCfg.name.trim() !== "Mon établissement");
      const inferredOrgKind =
        identityCfg.organizationKind ??
        (activeCount >= 2 ? "groupe" : activeCount === 1 ? "standalone" : undefined);
      setIdentity({ ...identityCfg, organizationKind: inferredOrgKind });
      setEstablishments(loadedEstablishments);
      setExistingConfigDetected(hasRealName || activeCount >= 2);
      setNotifications(cfg.notifications || {});
      setTravels(cfg.travels || { transportProviders: [] });
      const integrationsCfg = cfg.integrations || {};
      setIntegrations({
        ...integrationsCfg,
        microsoftOneDrive: {
          enabled:
            integrationsCfg.microsoftOneDrive?.enabled ??
            (hasRealName || activeCount >= 2),
        },
      });
      setExternalLinks(cfg.externalLinks || []);
      setWantQuickLinks((cfg.externalLinks || []).length > 0);
      const onboardingStep = normalizeOnboardingStep(identityCfg.onboardingStep || 1);
      const onboardingCompleted = identityCfg.onboardingCompleted === true;

      if (!reviewMode && onboardingCompleted) {
        router.replace("/dashboard");
        return;
      }

      if (
        !reviewMode &&
        !onboardingCompleted &&
        onboardingStep >= TOTAL_STEPS &&
        (hasRealName || activeCount >= 2)
      ) {
        const completeRes = await fetch("/api/settings/onboarding/complete", { method: "PUT" });
        if (completeRes.ok) {
          router.replace("/dashboard");
          return;
        }
      }

      setStep(onboardingStep);
      setHasInternat(Boolean(cfg.notifications?.internatRollCallRecipients || cfg.notifications?.internatEmergencyRecipients?.length));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [reviewMode, router]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (step !== 3) return;
    let cancelled = false;
    setMembersLoading(true);
    fetch("/api/members")
      .then((res) => res.json().then((j) => ({ ok: res.ok, j })))
      .then(({ ok, j }) => {
        if (cancelled) return;
        if (ok) setClerkMembers((j.users || []) as ClerkMemberOption[]);
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) setMembersLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [step]);

  const saveSite = async (patch: Partial<SiteIdentity>, nextStep?: number) => {
    const payload = { ...identity, ...patch, ...(nextStep ? { onboardingStep: nextStep } : {}) };
    const res = await fetch("/api/settings/site", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement identité impossible");
    setIdentity(j.config?.identity || payload);
  };

  const saveEstablishmentsApi = async (list: EstablishmentDraft[], nextStep?: number) => {
    const parsed = list.map((e) => ({
      id: e.id,
      label: e.label,
      kind: e.kind,
      directorName: e.directorName,
      directorEmail: e.directorEmail,
      directorClerkUserId: e.directorClerkUserId,
      grades: e.grades,
      clerkRoleSlugs: clerkRoleSlugsForEstablishment(e),
      active: e.active !== false,
    }));
    const res = await fetch("/api/settings/establishments", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ establishments: parsed }),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement établissements impossible");
    if (nextStep) await saveSite({}, nextStep);
    setEstablishments(list);
  };

  const saveNotificationsApi = async (patch: Partial<NotificationsConfig>, nextStep?: number) => {
    const payload = { ...notifications, ...patch };
    const res = await fetch("/api/settings/notifications", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement notifications impossible");
    setNotifications(j.config?.notifications || payload);
    if (nextStep) await saveSite({}, nextStep);
  };

  const saveTravelsApi = async (patch: Partial<TravelsModuleConfig>, nextStep?: number) => {
    const payload = { ...travels, ...patch };
    const res = await fetch("/api/settings/travels", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement voyages impossible");
    setTravels(j.travels || payload);
    if (nextStep) await saveSite({}, nextStep);
  };

  const saveIntegrationsApi = async (patch: IntegrationsConfig, nextStep?: number) => {
    const payload = { ...integrations, ...patch };
    const res = await fetch("/api/settings/integrations", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement intégrations impossible");
    setIntegrations(j.integrations || payload);
    if (nextStep) await saveSite({}, nextStep);
  };

  const saveExternalLinksApi = async (links: ExternalQuickLinkConfig[], nextStep?: number) => {
    const valid = links.filter((l) => l.name.trim() && l.link.trim());
    const res = await fetch("/api/settings/external-links", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ links: valid }),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement liens impossible");
    setExternalLinks(j.links || valid);
    if (nextStep) await saveSite({}, nextStep);
  };

  const geocodeAddress = async () => {
    const res = await fetch("/api/settings/geocode", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        street: identity.address?.street,
        zip: identity.address?.zip,
        city: identity.address?.city,
      }),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Géocodage impossible");
    setIdentity((prev) => ({
      ...prev,
      address: {
        ...prev.address,
        latitude: j.latitude,
        longitude: j.longitude,
      },
    }));
    return j;
  };

  const activeEstablishmentKinds = useMemo(() => {
    return new Set(
      establishments
        .filter((e) => e.active !== false)
        .map((e) => e.kind)
        .filter(Boolean) as EstablishmentKind[],
    );
  }, [establishments]);

  const goNext = async () => {
    setSaving(true);
    setError(null);
    try {
      const next = Math.min(TOTAL_STEPS, step + 1);
      if (step === 1) {
        if (!identity.name?.trim()) throw new Error("Le nom de la plateforme est requis.");
        await saveSite(
          {
            name: identity.name,
            organizationKind: identity.organizationKind,
            shortName: identity.shortName || identity.name,
          },
          next,
        );
      } else if (step === 2) {
        await saveSite({ dashboardAccent: identity.dashboardAccent, shortName: identity.shortName }, next);
      } else if (step === 3) {
        if (establishments.length === 0) throw new Error("Ajoutez au moins un établissement.");
        if (identity.organizationKind === "standalone" && establishments.length > 1) {
          throw new Error("Pour un établissement unique, ne conservez qu'un seul niveau.");
        }
        await saveEstablishmentsApi(establishments, next);
      } else if (step === 4) {
        if (!identity.address?.street?.trim() || !identity.address?.city?.trim()) {
          throw new Error("Renseignez au minimum la rue et la ville (code postal recommandé).");
        }
        let latitude: number | undefined;
        let longitude: number | undefined;
        try {
          const geo = await geocodeAddress();
          latitude = geo.latitude;
          longitude = geo.longitude;
        } catch {
          /* le serveur tentera le géocodage à l'enregistrement */
        }
        await saveSite(
          {
            address: {
              ...identity.address,
              ...(latitude != null && longitude != null ? { latitude, longitude } : {}),
            },
          },
          next,
        );
      } else if (step === 5) {
        await saveSite({ assistanceEmail: PLATFORM_ASSISTANCE_EMAIL }, next);
        await saveNotificationsApi(
          {
            hseOps: notifications.hseOps,
            photocopiesOps: notifications.photocopiesOps,
          },
          next,
        );
      } else if (step === 6) {
        await saveNotificationsApi({
          travelsCompta: notifications.travelsCompta,
          travelsCuisine: notifications.travelsCuisine,
        });
        await saveTravelsApi({ transportProviders: travels.transportProviders || [] }, next);
      } else if (step === 7) {
        await saveIntegrationsApi({ zeendoc: integrations.zeendoc }, next);
        if (integrations.zeendoc?.destinationEmail) {
          await saveNotificationsApi({ travelsZeendoc: integrations.zeendoc.destinationEmail });
        }
      } else if (step === 8) {
        await saveNotificationsApi(
          {
            absencesNotifyProfEcole: notifications.absencesNotifyProfEcole,
            absencesNotifyProfCollege: notifications.absencesNotifyProfCollege,
            absencesNotifyProfLycee: notifications.absencesNotifyProfLycee,
            absencesNotifyOgecCompta: notifications.absencesNotifyOgecCompta,
          },
          next,
        );
      } else if (step === 9) {
        if (hasInternat) {
          await saveNotificationsApi(
            {
              internatRollCallRecipients: notifications.internatRollCallRecipients,
              internatEmergencyRecipients: notifications.internatEmergencyRecipients,
            },
            next,
          );
        } else {
          await saveSite({}, next);
        }
      } else if (step === 10) {
        await saveExternalLinksApi(wantQuickLinks ? externalLinks : [], next);
      } else if (step === 11) {
        await saveIntegrationsApi(
          {
            microsoftOneDrive: {
              enabled: integrations.microsoftOneDrive?.enabled ?? existingConfigDetected,
            },
          },
          next,
        );
      } else if (step === 12) {
        const res = await fetch("/api/settings/onboarding/complete", { method: "PUT" });
        const j = await res.json();
        if (!res.ok) throw new Error(j.error || "Finalisation impossible");
        router.push(reviewMode ? "/parametres" : "/onboarding/microsoft");
        return;
      }
      setStep(next);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const goPrev = () => setStep((s) => Math.max(1, s - 1));

  const addPresetEstablishment = (preset: (typeof ESTABLISHMENT_KIND_PRESETS)[number]) => {
    if (establishments.some((e) => e.id === preset.id)) return;
    if (identity.organizationKind === "standalone" && establishments.length >= 1) return;
    setEstablishments((prev) => [
      ...prev,
      {
        id: preset.id,
        label: preset.label,
        kind: preset.kind,
        grades: preset.grades,
        directorName: "",
        directorEmail: "",
        directorClerkUserId: "",
        clerkRoleSlugs: clerkRoleSlugsForEstablishment(preset),
        active: true,
      },
    ]);
  };

  const stepTitle = useMemo(() => {
    const titles: Record<number, string> = {
      1: "Bienvenue — votre établissement",
      2: "Identité visuelle",
      3: "Établissements",
      4: "Adresse & météo",
      5: "Alertes HSE & photocopies",
      6: "Sorties scolaires",
      7: "Envoi documents (Zeendoc / mail)",
      8: "Absences",
      9: "Internat",
      10: "Raccourcis tableau de bord",
      11: "OneDrive / OCR",
      12: "Récapitulatif",
    };
    return titles[step] || "";
  }, [step]);

  if (loading) {
    return <div className="p-12 text-center text-stone-500">Chargement de la configuration…</div>;
  }

  return (
    <div className="min-h-screen bg-gradient-to-b from-[#F7F4EF] via-white to-[#EEF5F0] py-10 px-4">
      <div className="max-w-2xl mx-auto">
        <div className="mb-8">
          <p className="text-xs font-semibold uppercase tracking-wider text-[#2F6B4A] mb-1">
            Configuration {reviewMode ? "— relecture" : "initiale"} · Étape {step}/{TOTAL_STEPS}
          </p>
          <h1 className="text-2xl font-black text-[#14231A]">
            <span className={SCOLA_GRADIENT_TEXT}>{stepTitle}</span>
          </h1>
          <div className="mt-4 h-2 rounded-full bg-stone-200 overflow-hidden">
            <div
              className="h-full bg-[#2F6B4A] transition-all"
              style={{ width: `${(step / TOTAL_STEPS) * 100}%` }}
            />
          </div>
        </div>

        <div className="bg-white/90 rounded-2xl border border-stone-200 shadow-sm p-6 md:p-8">
          {error && <div className="mb-4 text-sm text-red-700 bg-red-50 border border-red-200 rounded-xl p-3">{error}</div>}
          {existingConfigDetected && !reviewMode && (
            <div className="mb-4 text-sm text-amber-900 bg-amber-50 border border-amber-200 rounded-xl p-3">
              Configuration existante détectée (nom de plateforme ou plusieurs établissements). Les champs sont
              pré-remplis depuis S3 — vérifiez avant d&apos;enregistrer chaque étape.
            </div>
          )}

          {step === 1 && (
            <>
              <Help>
                Indiquez si vous gérez un seul établissement ou un groupe scolaire (plusieurs niveaux). Cela adapte les
                modules (sorties, absences, tableau de bord).
              </Help>
              <Field label="Nom affiché de la plateforme">
                <input className={inputClass} value={identity.name || ""} onChange={(e) => setIdentity({ ...identity, name: e.target.value })} />
              </Field>
              <Field label="Type d'organisation">
                <select
                  className={inputClass}
                  value={identity.organizationKind || "standalone"}
                  onChange={(e) =>
                    setIdentity({ ...identity, organizationKind: e.target.value as "standalone" | "groupe" })
                  }
                >
                  <option value="standalone">Un seul établissement</option>
                  <option value="groupe">Groupe scolaire (plusieurs niveaux)</option>
                </select>
              </Field>
            </>
          )}

          {step === 2 && (
            <>
              <Help>
                Personnalisez l&apos;apparence du tableau de bord. Le logo de l&apos;établissement est
                configuré par l&apos;administrateur Scola (portail plateforme).
              </Help>
              <Field label="Nom court">
                <input className={inputClass} value={identity.shortName || ""} onChange={(e) => setIdentity({ ...identity, shortName: e.target.value })} />
              </Field>
              <Field label="Couleur d'accent">
                <select
                  className={inputClass}
                  value={identity.dashboardAccent || "green"}
                  onChange={(e) => setIdentity({ ...identity, dashboardAccent: e.target.value })}
                >
                  {DASHBOARD_ACCENT_OPTIONS.map((o) => (
                    <option key={o.id} value={o.id}>
                      {o.label}
                    </option>
                  ))}
                </select>
              </Field>
            </>
          )}

          {step === 3 && (
            <>
              <Help>
                Ajoutez les établissements actifs (liste vide au départ). Choisissez le responsable dans le
                personnel Clerk : le rôle direction correspondant est assigné automatiquement à l&apos;enregistrement.
              </Help>
              <div className="flex flex-wrap gap-2 mb-4">
                {ESTABLISHMENT_KIND_PRESETS.filter((p) => !establishments.some((e) => e.id === p.id)).map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    disabled={identity.organizationKind === "standalone" && establishments.length >= 1}
                    className="text-sm px-3 py-1.5 rounded-lg border border-[#2F6B4A]/30 text-[#2F6B4A] hover:bg-emerald-50 disabled:opacity-40"
                    onClick={() => addPresetEstablishment(p)}
                  >
                    + {p.label}
                  </button>
                ))}
              </div>
              {identity.organizationKind === "standalone" && (
                <p className="text-xs text-stone-500 mb-4">
                  Établissement unique : un seul niveau (école, collège, lycée ou autre).
                </p>
              )}
              {establishments.length === 0 && (
                <p className="text-sm text-stone-500 mb-4">Aucun établissement pour l&apos;instant. Utilisez les boutons ci-dessus.</p>
              )}
              {establishments.map((e, idx) => (
                <div key={e.id} className="mb-6 p-4 rounded-xl border border-stone-200 bg-stone-50/50">
                  <div className="flex justify-between items-center mb-2">
                    <strong>{e.label}</strong>
                    <button type="button" className="text-xs text-red-600" onClick={() => setEstablishments((prev) => prev.filter((_, i) => i !== idx))}>
                      Retirer
                    </button>
                  </div>
                  <p className="text-xs text-stone-500 mb-3">
                    Rôle Clerk : {directionRoleForKind(e.kind || e.id)}
                  </p>
                  <Field label="Responsable (Clerk)">
                    <ClerkPersonSelect
                      members={clerkMembers}
                      selectedId={e.directorClerkUserId || ""}
                      loading={membersLoading}
                      onChange={(member) => {
                        const copy = [...establishments];
                        if (!member) {
                          copy[idx] = { ...copy[idx], directorClerkUserId: "" };
                        } else {
                          copy[idx] = {
                            ...copy[idx],
                            directorClerkUserId: member.clerkUserId,
                            directorName: memberDisplayName(member),
                            directorEmail: member.email,
                          };
                        }
                        setEstablishments(copy);
                      }}
                    />
                  </Field>
                  <Field label="Nom affiché (PDF)">
                    <input className={inputClass} value={e.directorName || ""} onChange={(ev) => {
                      const copy = [...establishments];
                      copy[idx] = { ...copy[idx], directorName: ev.target.value };
                      setEstablishments(copy);
                    }} />
                  </Field>
                  <Field label="E-mail direction">
                    <input className={inputClass} type="email" value={e.directorEmail || ""} onChange={(ev) => {
                      const copy = [...establishments];
                      copy[idx] = { ...copy[idx], directorEmail: ev.target.value };
                      setEstablishments(copy);
                    }} />
                  </Field>
                </div>
              ))}
            </>
          )}

          {step === 4 && (
            <>
              <Help>
                L'adresse exacte alimente le widget météo du tableau de bord. Nous géolocalisons automatiquement votre
                établissement à partir de la rue, du code postal et de la ville.
              </Help>
              <Field label="Rue">
                <input className={inputClass} value={identity.address?.street || ""} onChange={(e) => setIdentity({ ...identity, address: { ...identity.address, street: e.target.value } })} />
              </Field>
              <div className="grid grid-cols-2 gap-3">
                <Field label="Code postal">
                  <input className={inputClass} value={identity.address?.zip || ""} onChange={(e) => setIdentity({ ...identity, address: { ...identity.address, zip: e.target.value } })} />
                </Field>
                <Field label="Ville">
                  <input className={inputClass} value={identity.address?.city || ""} onChange={(e) => setIdentity({ ...identity, address: { ...identity.address, city: e.target.value } })} />
                </Field>
              </div>
              {(identity.address?.latitude != null && identity.address?.longitude != null) && (
                <p className="text-xs text-emerald-700 mb-2">
                  Coordonnées : {identity.address.latitude.toFixed(4)}, {identity.address.longitude.toFixed(4)}
                </p>
              )}
            </>
          )}

          {step === 5 && (
            <>
              <Help>
                Indiquez qui reçoit les e-mails après validation de la direction (demandes HSE et photocopies
                couleur). Ce ne sont pas nécessairement des « responsables » de service.
              </Help>
              <Field label="Gestionnaire HSE (e-mail)">
                <input className={inputClass} type="email" value={notifications.hseOps || ""} onChange={(e) => setNotifications({ ...notifications, hseOps: e.target.value })} />
              </Field>
              <Field label="Gestionnaire photocopies couleur (e-mail)">
                <input className={inputClass} type="email" value={notifications.photocopiesOps || ""} onChange={(e) => setNotifications({ ...notifications, photocopiesOps: e.target.value })} />
              </Field>
            </>
          )}

          {step === 6 && (
            <>
              <Help>
                Pour les sorties scolaires : compta reçoit les devis signés, la cuisine reçoit les commandes
                restauration, les transporteurs reçoivent les demandes de devis bus.
              </Help>
              <Field label="E-mails comptabilité (séparés par des virgules)">
                <input
                  className={inputClass}
                  value={Array.isArray(notifications.travelsCompta) ? notifications.travelsCompta.join(", ") : ""}
                  onChange={(e) =>
                    setNotifications({
                      ...notifications,
                      travelsCompta: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
                    })
                  }
                />
              </Field>
              <Field label="E-mail cuisine / prestataire restauration">
                <input className={inputClass} type="email" value={notifications.travelsCuisine || ""} onChange={(e) => setNotifications({ ...notifications, travelsCuisine: e.target.value })} />
              </Field>
              <p className="text-sm font-medium text-stone-700 mb-2">Transporteurs habituels</p>
              {(travels.transportProviders || []).map((p, idx) => (
                <div key={idx} className="grid grid-cols-2 gap-2 mb-2">
                  <input className={inputClass} placeholder="Nom" value={p.name} onChange={(e) => {
                    const copy = [...(travels.transportProviders || [])];
                    copy[idx] = { ...copy[idx], name: e.target.value };
                    setTravels({ ...travels, transportProviders: copy });
                  }} />
                  <input className={inputClass} placeholder="E-mail" type="email" value={p.email} onChange={(e) => {
                    const copy = [...(travels.transportProviders || [])];
                    copy[idx] = { ...copy[idx], email: e.target.value };
                    setTravels({ ...travels, transportProviders: copy });
                  }} />
                </div>
              ))}
              <button type="button" className="text-sm text-[#2F6B4A]" onClick={() => setTravels({ ...travels, transportProviders: [...(travels.transportProviders || []), { name: "", email: "" }] })}>
                + Ajouter un transporteur
              </button>
            </>
          )}

          {step === 7 && (
            <>
              <Help>
                Si vous utilisez Zeendoc, le bouton du module voyages portera ce nom et enverra les PDF à l'adresse
                configurée. Sinon, choisissez « Envoyer par mail ».
              </Help>
              <Field label="Utilisez-vous Zeendoc ?">
                <select
                  className={inputClass}
                  value={integrations.zeendoc?.enabled ? "yes" : "no"}
                  onChange={(e) =>
                    setIntegrations({
                      ...integrations,
                      zeendoc: {
                        enabled: e.target.value === "yes",
                        buttonLabel: e.target.value === "yes" ? "Envoyer sur Zeendoc" : "Envoyer par mail",
                        destinationEmail: integrations.zeendoc?.destinationEmail,
                      },
                    })
                  }
                >
                  <option value="no">Non — envoi par mail simple</option>
                  <option value="yes">Oui — Zeendoc</option>
                </select>
              </Field>
              <Field label="Libellé du bouton">
                <input className={inputClass} value={integrations.zeendoc?.buttonLabel || "Envoyer par mail"} onChange={(e) => setIntegrations({ ...integrations, zeendoc: { ...integrations.zeendoc, enabled: integrations.zeendoc?.enabled ?? false, buttonLabel: e.target.value } })} />
              </Field>
              <Field label="E-mail de destination des PDF">
                <input className={inputClass} type="email" value={integrations.zeendoc?.destinationEmail || notifications.travelsZeendoc || ""} onChange={(e) => setIntegrations({ ...integrations, zeendoc: { ...integrations.zeendoc, enabled: integrations.zeendoc?.enabled ?? false, destinationEmail: e.target.value } })} />
              </Field>
            </>
          )}

          {step === 8 && (
            <>
              <Help>
                Après validation par la direction, ces personnes reçoivent les notifications finales. Les champs
                affichés correspondent à vos établissements configurés à l&apos;étape 3.
              </Help>
              {activeEstablishmentKinds.has("ecole") && (
                <Field label="Professeurs — école (nom + e-mail)">
                  <input className={`${inputClass} mb-2`} placeholder="Nom" value={notifications.absencesNotifyProfEcole?.label || ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyProfEcole: { ...notifications.absencesNotifyProfEcole, label: e.target.value, email: notifications.absencesNotifyProfEcole?.email || "" } })} />
                  <input className={inputClass} placeholder="E-mail" type="email" value={notifications.absencesNotifyProfEcole?.email || ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyProfEcole: { label: notifications.absencesNotifyProfEcole?.label, email: e.target.value } })} />
                </Field>
              )}
              {activeEstablishmentKinds.has("college") && (
                <Field label="Professeurs — collège (nom + e-mail)">
                  <input className={`${inputClass} mb-2`} placeholder="Nom" value={notifications.absencesNotifyProfCollege?.label || notifications.absencesNotifyProfCollegeLycee?.label || ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyProfCollege: { ...notifications.absencesNotifyProfCollege, label: e.target.value, email: notifications.absencesNotifyProfCollege?.email || notifications.absencesNotifyProfCollegeLycee?.email || "" } })} />
                  <input className={inputClass} placeholder="E-mail" type="email" value={notifications.absencesNotifyProfCollege?.email || notifications.absencesNotifyProfCollegeLycee?.email || ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyProfCollege: { label: notifications.absencesNotifyProfCollege?.label || notifications.absencesNotifyProfCollegeLycee?.label, email: e.target.value } })} />
                </Field>
              )}
              {activeEstablishmentKinds.has("lycee") && (
                <Field label="Professeurs — lycée (nom + e-mail)">
                  <input className={`${inputClass} mb-2`} placeholder="Nom" value={notifications.absencesNotifyProfLycee?.label || notifications.absencesNotifyProfCollegeLycee?.label || ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyProfLycee: { ...notifications.absencesNotifyProfLycee, label: e.target.value, email: notifications.absencesNotifyProfLycee?.email || notifications.absencesNotifyProfCollegeLycee?.email || "" } })} />
                  <input className={inputClass} placeholder="E-mail" type="email" value={notifications.absencesNotifyProfLycee?.email || notifications.absencesNotifyProfCollegeLycee?.email || ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyProfLycee: { label: notifications.absencesNotifyProfLycee?.label || notifications.absencesNotifyProfCollegeLycee?.label, email: e.target.value } })} />
                </Field>
              )}
              {!activeEstablishmentKinds.has("ecole") &&
                !activeEstablishmentKinds.has("college") &&
                !activeEstablishmentKinds.has("lycee") && (
                  <p className="text-sm text-amber-800 bg-amber-50 border border-amber-200 rounded-xl p-3">
                    Ajoutez au moins un établissement (étape 3) pour configurer les notifications professeurs.
                  </p>
                )}
              <Field label="Personnel OGEC, administratif & RH (e-mails séparés par des virgules)">
                <input className={inputClass} value={Array.isArray(notifications.absencesNotifyOgecCompta) ? notifications.absencesNotifyOgecCompta.join(", ") : ""} onChange={(e) => setNotifications({ ...notifications, absencesNotifyOgecCompta: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
              </Field>
            </>
          )}

          {step === 9 && (
            <>
              <Help>Configurez l'internat si vous hébergez des élèves. Sinon, passez à l'étape suivante.</Help>
              <label className="flex items-center gap-2 mb-4 text-sm">
                <input type="checkbox" checked={hasInternat} onChange={(e) => setHasInternat(e.target.checked)} />
                Nous avons un internat
              </label>
              {hasInternat && (
                <>
                  <Field label="Qui reçoit l'appel ? (e-mail)">
                    <input
                      className={inputClass}
                      type="email"
                      value={
                        notifications.internatRollCallRecipients?.appelContact ||
                        notifications.internatRollCallRecipients?.directionLycee ||
                        ""
                      }
                      onChange={(e) =>
                        setNotifications({
                          ...notifications,
                          internatRollCallRecipients: {
                            ...notifications.internatRollCallRecipients,
                            appelContact: e.target.value,
                          },
                        })
                      }
                    />
                  </Field>
                  <Field label="Urgences internat (e-mails séparés par des virgules)">
                    <input className={inputClass} value={Array.isArray(notifications.internatEmergencyRecipients) ? notifications.internatEmergencyRecipients.join(", ") : ""} onChange={(e) => setNotifications({ ...notifications, internatEmergencyRecipients: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
                  </Field>
                </>
              )}
            </>
          )}

          {step === 10 && (
            <>
              <Help>
                Ajoutez des raccourcis sous le tableau de bord (nom, URL, image). Utile pour ÉcoleDirecte, Zeendoc,
                Arena… ou laissez vide si vous n&apos;en avez pas besoin.
              </Help>
              <label className="flex items-center gap-2 mb-4 text-sm">
                <input
                  type="checkbox"
                  checked={wantQuickLinks}
                  onChange={(e) => {
                    setWantQuickLinks(e.target.checked);
                    if (!e.target.checked) setExternalLinks([]);
                  }}
                />
                Souhaitez-vous ajouter des raccourcis ?
              </label>
              {wantQuickLinks && (
                <div className="space-y-4">
                  {externalLinks.map((link, idx) => (
                    <div key={link.id} className="p-4 rounded-xl border border-stone-200 bg-stone-50/50 space-y-2">
                      <div className="flex justify-between items-center">
                        <span className="text-xs font-bold text-stone-500">Raccourci {idx + 1}</span>
                        <button
                          type="button"
                          className="text-xs text-red-600"
                          onClick={() => setExternalLinks((prev) => prev.filter((_, i) => i !== idx))}
                        >
                          Retirer
                        </button>
                      </div>
                      <input
                        className={inputClass}
                        placeholder="Nom (ex. École Directe)"
                        value={link.name}
                        onChange={(e) => {
                          const copy = [...externalLinks];
                          copy[idx] = { ...copy[idx], name: e.target.value };
                          setExternalLinks(copy);
                        }}
                      />
                      <input
                        className={inputClass}
                        placeholder="URL du lien"
                        type="url"
                        value={link.link}
                        onChange={(e) => {
                          const copy = [...externalLinks];
                          copy[idx] = { ...copy[idx], link: e.target.value };
                          setExternalLinks(copy);
                        }}
                      />
                      <input
                        className={inputClass}
                        placeholder="URL de l'image (optionnel)"
                        type="url"
                        value={link.img || ""}
                        onChange={(e) => {
                          const copy = [...externalLinks];
                          copy[idx] = { ...copy[idx], img: e.target.value };
                          setExternalLinks(copy);
                        }}
                      />
                    </div>
                  ))}
                  <button
                    type="button"
                    className="text-sm text-[#2F6B4A]"
                    onClick={() =>
                      setExternalLinks((prev) => [
                        ...prev,
                        newQuickLinkSlot(prev.length),
                      ])
                    }
                  >
                    + Ajouter un raccourci
                  </button>
                </div>
              )}
            </>
          )}

          {step === 11 && (
            <>
              <Help>
                Le module « Ajout documents IA » utilise OneDrive. Activez-le si vous déposez les dossiers élèves sur
                OneDrive. Les identifiants Azure sont configurés côté hébergeur (secrets tenant).
              </Help>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={integrations.microsoftOneDrive?.enabled ?? false}
                  onChange={(e) =>
                    setIntegrations({
                      ...integrations,
                      microsoftOneDrive: { enabled: e.target.checked },
                    })
                  }
                />
                Nous utilisons OneDrive pour les dossiers élèves
              </label>
            </>
          )}

          {step === 12 && (
            <>
              <Help>Vérifiez les informations saisies. Vous pourrez tout modifier ultérieurement dans Paramètres généraux.</Help>
              <ul className="text-sm text-stone-700 space-y-2">
                <li><strong>Organisation :</strong> {identity.name} ({identity.organizationKind === "groupe" ? "groupe scolaire" : "établissement unique"})</li>
                <li><strong>Établissements :</strong> {establishments.map((e) => e.label).join(", ") || "—"}</li>
                <li><strong>Adresse :</strong> {identity.address?.street}, {identity.address?.zip} {identity.address?.city}</li>
                <li><strong>Transporteurs :</strong> {(travels.transportProviders || []).length}</li>
                <li><strong>Raccourcis :</strong> {wantQuickLinks ? externalLinks.filter((l) => l.name && l.link).length : 0}</li>
                <li><strong>Licences Microsoft :</strong> à configurer à l&apos;étape suivante (A3 référents + A1 enseignants)</li>
              </ul>
            </>
          )}

          <div className="flex justify-between mt-8 pt-6 border-t border-stone-100">
            <button type="button" className="px-4 py-2 text-sm text-stone-600 disabled:opacity-40" disabled={step <= 1 || saving} onClick={goPrev}>
              Précédent
            </button>
            <button
              type="button"
              className="px-5 py-2.5 rounded-xl bg-[#2F6B4A] text-white text-sm font-semibold disabled:opacity-50 hover:bg-[#255A3D]"
              disabled={saving}
              onClick={goNext}
            >
              {saving ? "Enregistrement…" : step === TOTAL_STEPS ? (reviewMode ? "Terminer la relecture" : "Continuer — licences Microsoft") : "Suivant"}
            </button>
          </div>
          {reviewMode && step === TOTAL_STEPS && (
            <button type="button" className="mt-3 text-sm text-[#2F6B4A] w-full text-center" onClick={() => router.push("/parametres")}>
              Retour aux paramètres sans finaliser
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

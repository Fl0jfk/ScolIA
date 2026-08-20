"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type {
  Establishment,
  EstablishmentKind,
  ExternalQuickLinkConfig,
  IntegrationsConfig,
  NotificationsConfig,
  SiteIdentity,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import { clerkRoleSlugsForEstablishment } from "@/app/lib/establishment-catalog";
import {
  normalizeOnboardingStep,
  ONBOARDING_WIZARD_VERSION,
  TOTAL_CHAPTERS,
} from "@/app/lib/onboarding-chapters";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import OnboardingShell from "@/app/components/onboarding/OnboardingShell";
import ChapterWelcome from "@/app/components/onboarding/chapters/ChapterWelcome";
import ChapterIdentity from "@/app/components/onboarding/chapters/ChapterIdentity";
import ChapterStructure from "@/app/components/onboarding/chapters/ChapterStructure";
import ChapterContacts from "@/app/components/onboarding/chapters/ChapterContacts";
import ChapterReview from "@/app/components/onboarding/chapters/ChapterReview";
import { dash } from "@/app/lib/dashboard-brand";

export default function OnboardingWizard() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const reviewMode = searchParams.get("review") === "1";

  const [step, setStep] = useState(1);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [identity, setIdentity] = useState<Partial<SiteIdentity>>({});
  const [establishments, setEstablishments] = useState<Establishment[]>([]);
  const [notifications, setNotifications] = useState<Partial<NotificationsConfig>>({});
  const [travels, setTravels] = useState<Partial<TravelsModuleConfig>>({ transportProviders: [] });
  const [integrations, setIntegrations] = useState<IntegrationsConfig>({});
  const [externalLinks, setExternalLinks] = useState<ExternalQuickLinkConfig[]>([]);
  const [wantQuickLinks, setWantQuickLinks] = useState(false);
  const [hasInternat, setHasInternat] = useState(false);
  const [existingConfigDetected, setExistingConfigDetected] = useState(false);
  const [clerkMembers, setClerkMembers] = useState<ClerkMemberOption[]>([]);
  const [membersLoading, setMembersLoading] = useState(false);

  const patchIdentity = (patch: Partial<SiteIdentity>) =>
    setIdentity((prev) => ({ ...prev, ...patch }));

  const loadConfig = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings");
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      const cfg = j.config;
      const loadedEstablishments = (cfg.establishments || []) as Establishment[];
      const identityCfg = (cfg.identity || {}) as SiteIdentity;
      const activeCount = loadedEstablishments.filter((e) => e.active !== false).length;
      const hasRealName = Boolean(
        identityCfg.name?.trim() && identityCfg.name.trim() !== "Mon établissement",
      );
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
            integrationsCfg.microsoftOneDrive?.enabled ?? (hasRealName || activeCount >= 2),
        },
      });
      setExternalLinks(cfg.externalLinks || []);
      setWantQuickLinks((cfg.externalLinks || []).length > 0);
      const onboardingStep = normalizeOnboardingStep(
        identityCfg.onboardingStep || 1,
        identityCfg.onboardingWizardVersion,
      );
      const onboardingCompleted = identityCfg.onboardingCompleted === true;

      if (!reviewMode && onboardingCompleted) {
        router.replace("/dashboard");
        return;
      }

      if (
        !reviewMode &&
        !onboardingCompleted &&
        onboardingStep >= TOTAL_CHAPTERS &&
        (hasRealName || activeCount >= 2)
      ) {
        const completeRes = await fetch("/api/settings/onboarding/complete", { method: "PUT" });
        if (completeRes.ok) {
          router.replace("/dashboard");
          return;
        }
      }

      setStep(onboardingStep);
      setHasInternat(
        Boolean(
          cfg.notifications?.internatRollCallRecipients ||
            cfg.notifications?.internatEmergencyRecipients?.length,
        ),
      );
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
    const payload = {
      ...identity,
      ...patch,
      onboardingWizardVersion: ONBOARDING_WIZARD_VERSION,
      ...(nextStep ? { onboardingStep: nextStep } : {}),
    };
    const res = await fetch("/api/settings/site", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Enregistrement identité impossible");
    setIdentity(j.config?.identity || payload);
  };

  const saveEstablishmentsApi = async (list: Establishment[], nextStep?: number) => {
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

  const goPrev = () => {
    setError(null);
    setStep((s) => Math.max(1, s - 1));
  };

  const goNext = async () => {
    setSaving(true);
    setError(null);
    try {
      const next = Math.min(TOTAL_CHAPTERS, step + 1);
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
            dashboardAccent: identity.dashboardAccent,
            shortName: identity.shortName || identity.name,
            address: {
              ...identity.address,
              ...(latitude != null ? { latitude } : {}),
              ...(longitude != null ? { longitude } : {}),
            },
          },
          next,
        );
      } else if (step === 3) {
        if (establishments.length === 0) throw new Error("Ajoutez au moins un établissement.");
        if (identity.organizationKind === "standalone" && establishments.length > 1) {
          throw new Error("Pour un établissement unique, ne conservez qu'un seul niveau.");
        }
        await saveEstablishmentsApi(establishments, next);
      } else if (step === 4) {
        const notifPatch: Partial<NotificationsConfig> = { ...notifications };
        if (!hasInternat) {
          notifPatch.internatRollCallRecipients = undefined;
          notifPatch.internatEmergencyRecipients = undefined;
        }
        if (integrations.zeendoc?.destinationEmail) {
          notifPatch.travelsZeendoc = integrations.zeendoc.destinationEmail;
        }
        await saveNotificationsApi(notifPatch);
        await saveTravelsApi(travels);
        await saveIntegrationsApi(integrations);
        await saveExternalLinksApi(wantQuickLinks ? externalLinks : [], next);
      } else if (step === 5) {
        if (reviewMode) {
          await saveSite({ onboardingStep: TOTAL_CHAPTERS });
          router.push("/parametres");
          return;
        }
        const completeRes = await fetch("/api/settings/onboarding/complete", { method: "PUT" });
        const cj = await completeRes.json();
        if (!completeRes.ok) throw new Error(cj.error || "Finalisation impossible");
        router.push("/onboarding/microsoft");
        return;
      }
      setStep(next);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="dashboard-themed flex min-h-screen items-center justify-center">
        <p className={`text-sm ${dash.textMid}`}>Chargement de la configuration…</p>
      </div>
    );
  }

  const footer = (
    <>
      <button
        type="button"
        className={`rounded-2xl border border-white/70 bg-white/60 px-4 py-2.5 text-sm font-semibold disabled:opacity-40 ${dash.ink}`}
        disabled={step <= 1 || saving}
        onClick={goPrev}
      >
        Précédent
      </button>
      <div className="flex flex-col items-end gap-2">
        <button
          type="button"
          className={`rounded-2xl px-6 py-2.5 text-sm shadow-lg shadow-[color:var(--dash-primary)]/25 disabled:opacity-50 ${dash.btnPrimaryGrad}`}
          disabled={saving}
          onClick={goNext}
        >
          {saving
            ? "Enregistrement…"
            : step === TOTAL_CHAPTERS
              ? reviewMode
                ? "Terminer la relecture"
                : "Continuer — licences Microsoft"
              : step === 1
                ? "Commencer"
                : "Continuer"}
        </button>
        {reviewMode && step === TOTAL_CHAPTERS ? (
          <button
            type="button"
            className={`text-sm ${dash.linkBold}`}
            onClick={() => router.push("/parametres")}
          >
            Retour aux paramètres sans finaliser
          </button>
        ) : null}
      </div>
    </>
  );

  return (
    <OnboardingShell
      chapter={step}
      reviewMode={reviewMode}
      accent={identity.dashboardAccent}
      error={error}
      banner={
        existingConfigDetected && !reviewMode && step === 1 ? (
          <div className="mb-4 rounded-2xl border border-amber-200/80 bg-amber-50/90 px-4 py-3 text-sm text-amber-950">
            Configuration existante détectée. Les champs sont pré-remplis — vérifiez avant
            d&apos;enregistrer chaque chapitre.
          </div>
        ) : null
      }
      footer={footer}
    >
      {step === 1 && (
        <ChapterWelcome identity={identity} onChange={patchIdentity} isHero={!reviewMode} />
      )}
      {step === 2 && <ChapterIdentity identity={identity} onChange={patchIdentity} />}
      {step === 3 && (
        <ChapterStructure
          identity={identity}
          establishments={establishments}
          onChange={setEstablishments}
          clerkMembers={clerkMembers}
          membersLoading={membersLoading}
        />
      )}
      {step === 4 && (
        <ChapterContacts
          notifications={notifications}
          setNotifications={setNotifications}
          travels={travels}
          setTravels={setTravels}
          integrations={integrations}
          setIntegrations={setIntegrations}
          externalLinks={externalLinks}
          setExternalLinks={setExternalLinks}
          wantQuickLinks={wantQuickLinks}
          setWantQuickLinks={setWantQuickLinks}
          hasInternat={hasInternat}
          setHasInternat={setHasInternat}
          activeEstablishmentKinds={activeEstablishmentKinds}
        />
      )}
      {step === 5 && (
        <ChapterReview
          identity={identity}
          establishments={establishments}
          travels={travels}
          wantQuickLinks={wantQuickLinks}
          externalLinks={externalLinks}
        />
      )}
    </OnboardingShell>
  );
}

import {
  parseDomainPlanningModule,
  parseEstablishment,
  parseEstablishmentsFile,
  parseExternalLinksFile,
  parseIntegrations,
  parseInternatModule,
  parseNotifications,
  parseProfRoomModule,
  parseClassAllocationSettings,
  parseStaffDirectoryFile,
  parseSiteIdentity,
  parseTravelsModule,
  type AppConfigBundle,
  type DomainPlanningModuleConfig,
  type Establishment,
  type ExternalQuickLinkConfig,
  type IntegrationsConfig,
  type InternatModuleConfig,
  type NotificationsConfig,
  type ProfRoomModuleConfig,
  type SiteIdentity,
  type StaffDirectoryRow,
  type TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import {
  defaultDomainPlanningModule,
  defaultEstablishments,
  defaultExternalLinks,
  defaultIntegrations,
  defaultInternatModule,
  defaultNotifications,
  defaultProfRoomModule,
  defaultClassAllocationSettings,
  defaultSiteIdentity,
  defaultStaffDirectory,
  defaultTravelsModule,
} from "@/app/lib/app-config-defaults";
import {
  laprovidenceEstablishments,
  laprovidenceExternalLinks,
  laprovidenceIntegrations,
  laprovidenceNotifications,
  laprovidenceSiteIdentity,
  LAPROVIDENCE_STAFF_DIRECTORY,
  laprovidenceTravelsModule,
} from "@/app/lib/laprovidence-seed";
import { normalizeDomainPlanningModule } from "@/app/lib/domain-planning-defaults";
import { withDefaultProfRoomSubjects } from "@/app/lib/prof-room-defaults";
import { getActiveEstablishments, shouldShowGroupeScolaire } from "@/app/lib/app-config-establishments";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { saveOrganigramConfig, laprovidenceOrganigramConfig } from "@/app/lib/organigramme-config";

const CACHE_MS = 45_000;
let cache: { at: number; bundle: AppConfigBundle; allEstablishments: Establishment[] } | null = null;

export function invalidateAppConfigCache() {
  cache = null;
}

function isOnboardingComplete(config: AppConfigBundle): boolean {
  return config.identity.onboardingCompleted === true;
}

/** Détection heuristique du tenant La Providence (migration legacy). */
export function looksLikeLaProvidenceTenant(identity: SiteIdentity): boolean {
  const name = identity.name?.toLowerCase() ?? "";
  return name.includes("providence") || name.includes("nicolas barré") || name.includes("nicolas barre");
}

function isLegacyTenantIdentity(identity: SiteIdentity, activeEstablishmentCount: number): boolean {
  const name = identity.name?.trim();
  const hasRealName = Boolean(name && name !== "Mon établissement");
  return hasRealName || activeEstablishmentCount >= 2;
}

/** Rétablit école/collège/lycée si site.json existe mais establishments.json jamais créé (La Providence). */
async function maybeBackfillLegacyEstablishments(
  estRaw: { data: unknown } | null,
  identity: SiteIdentity,
): Promise<Establishment[]> {
  if (estRaw?.data) return parseEstablishmentsFile(estRaw.data);
  if (!looksLikeLaProvidenceTenant(identity)) return defaultEstablishments();
  const est = laprovidenceEstablishments();
  try {
    await putJson("settings/establishments.json", { establishments: est });
    invalidateAppConfigCache();
  } catch (e) {
    console.error("[app-config] backfill establishments", e);
  }
  return est;
}

/** Rétablit transporteurs (et compta voyages) si travels.json absent ou vide (La Providence). */
async function maybeBackfillLegacyTravelsModule(
  identity: SiteIdentity,
  travels: TravelsModuleConfig,
): Promise<TravelsModuleConfig> {
  if (!looksLikeLaProvidenceTenant(identity)) return travels;
  if (travels.transportProviders.length > 0) return travels;

  const seed = laprovidenceTravelsModule();
  const merged: TravelsModuleConfig = {
    ...travels,
    transportProviders: seed.transportProviders,
    comptaEmails: travels.comptaEmails.length ? travels.comptaEmails : seed.comptaEmails,
    pdfFooterText: travels.pdfFooterText || seed.pdfFooterText,
    showGroupeScolaireOption: travels.showGroupeScolaireOption || seed.showGroupeScolaireOption,
  };
  try {
    await putJson("settings/modules/travels.json", merged);
    invalidateAppConfigCache();
  } catch (e) {
    console.error("[app-config] backfill travels", e);
  }
  return merged;
}

/** Réactive OneDrive / Zeendoc si integrations.json absent ou désactivé par l'onboarding. */
async function maybeBackfillLegacyIntegrations(
  identity: SiteIdentity,
  integrations: IntegrationsConfig,
): Promise<IntegrationsConfig> {
  if (!looksLikeLaProvidenceTenant(identity)) return integrations;

  const seed = laprovidenceIntegrations();
  const merged: IntegrationsConfig = { ...integrations };

  if (merged.microsoftOneDrive?.enabled !== true) {
    merged.microsoftOneDrive = { enabled: true };
  }
  if (merged.zeendoc?.enabled !== true) {
    merged.zeendoc = { ...seed.zeendoc, ...merged.zeendoc, enabled: true };
  }

  const unchanged =
    integrations.microsoftOneDrive?.enabled === merged.microsoftOneDrive?.enabled &&
    integrations.zeendoc?.enabled === merged.zeendoc?.enabled &&
    integrations.zeendoc?.destinationEmail === merged.zeendoc?.destinationEmail;
  if (unchanged) return integrations;

  try {
    await putJson("settings/integrations.json", merged);
    invalidateAppConfigCache();
  } catch (e) {
    console.error("[app-config] backfill integrations", e);
  }
  return merged;
}

async function maybeMigrateLegacyOnboarding(
  identityRaw: { data: unknown } | null,
  identity: SiteIdentity,
  establishments: Establishment[],
): Promise<SiteIdentity> {
  if (!identityRaw?.data) return identity;
  if (identity.onboardingCompleted === true) return identity;

  const activeCount = getActiveEstablishments(establishments).length;
  const atRecapStep =
    (identity.onboardingWizardVersion ?? 0) >= 2
      ? (identity.onboardingStep ?? 0) >= 5
      : (identity.onboardingStep ?? 0) >= 12;
  const legacyTenant = isLegacyTenantIdentity(identity, activeCount);

  if (identity.onboardingCompleted === false) {
    const shouldCompleteAnyway =
      looksLikeLaProvidenceTenant(identity) || (legacyTenant && atRecapStep);
    if (!shouldCompleteAnyway) return identity;
  } else if (!legacyTenant) {
    return identity;
  }

  const migrated: SiteIdentity = {
    ...identity,
    onboardingCompleted: true,
    onboardingCompletedAt: new Date().toISOString(),
    assistanceEmail: PLATFORM_ASSISTANCE_EMAIL,
    organizationKind:
      identity.organizationKind ?? (activeCount >= 2 ? "groupe" : "standalone"),
  };
  try {
    await putJson("settings/site.json", migrated);
  } catch (e) {
    console.error("[app-config] migration onboarding", e);
  }
  return migrated;
}

function withInferredOrganizationKind(identity: SiteIdentity, establishments: Establishment[]): SiteIdentity {
  if (identity.organizationKind) return identity;
  const activeCount = getActiveEstablishments(establishments).length;
  if (activeCount >= 2) return { ...identity, organizationKind: "groupe" };
  if (activeCount === 1) return { ...identity, organizationKind: "standalone" };
  return identity;
}

export async function loadAppConfig(): Promise<AppConfigBundle> {
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.bundle;

  const [
    identityRaw,
    estRaw,
    notifRaw,
    staffRaw,
    travelsRaw,
    profRaw,
    domainRaw,
    internatRaw,
    integrationsRaw,
    externalLinksRaw,
  ] = await Promise.all([
    getJson<unknown>("settings/site.json"),
    getJson<unknown>("settings/establishments.json"),
    getJson<unknown>("settings/notifications.json"),
    getJson<unknown>("settings/staff-directory.json"),
    getJson<unknown>("settings/modules/travels.json"),
    getJson<unknown>("settings/modules/prof-room.json"),
    getJson<unknown>("settings/modules/domain-planning.json"),
    getJson<unknown>("settings/modules/internat.json"),
    getJson<unknown>("settings/integrations.json"),
    getJson<unknown>("settings/external-links.json"),
  ]);

  let identity = identityRaw?.data
    ? parseSiteIdentity(identityRaw.data, { allowEmptyName: true })
    : defaultSiteIdentity();
  const allEstablishments = await maybeBackfillLegacyEstablishments(estRaw, identity);
  identity = await maybeMigrateLegacyOnboarding(identityRaw, identity, allEstablishments);
  identity = withInferredOrganizationKind(identity, allEstablishments);
  const notifications = notifRaw?.data ? parseNotifications(notifRaw.data) : defaultNotifications();
  const staffDirectory = staffRaw?.data ? parseStaffDirectoryFile(staffRaw.data) : defaultStaffDirectory();
  let travels = travelsRaw?.data ? parseTravelsModule(travelsRaw.data) : defaultTravelsModule();
  travels = await maybeBackfillLegacyTravelsModule(identity, travels);
  const profRoomRaw = profRaw?.data ? parseProfRoomModule(profRaw.data) : defaultProfRoomModule();
  const profRoom = withDefaultProfRoomSubjects(profRoomRaw);
  const domainPlanningRaw = domainRaw?.data
    ? parseDomainPlanningModule(domainRaw.data)
    : defaultDomainPlanningModule();
  const domainPlanning = normalizeDomainPlanningModule(domainPlanningRaw);
  const internat = internatRaw?.data ? parseInternatModule(internatRaw.data) : defaultInternatModule();
  let integrations = integrationsRaw?.data ? parseIntegrations(integrationsRaw.data) : defaultIntegrations();
  integrations = await maybeBackfillLegacyIntegrations(identity, integrations);
  const externalLinks = externalLinksRaw?.data
    ? parseExternalLinksFile(externalLinksRaw.data)
    : defaultExternalLinks();

  const activeEstablishments = getActiveEstablishments(allEstablishments);
  travels = {
    ...travels,
    showGroupeScolaireOption: shouldShowGroupeScolaire(allEstablishments),
  };

  const bundle: AppConfigBundle = {
    identity,
    establishments: activeEstablishments,
    notifications,
    staffDirectory,
    travels,
    profRoom,
    domainPlanning,
    internat,
    integrations,
    externalLinks,
    classAllocation: defaultClassAllocationSettings(),
  };
  cache = { at: Date.now(), bundle, allEstablishments };
  return bundle;
}

/** Tous les sites (actifs et inactifs) — pour l’UI Paramètres / onboarding. */
export async function loadAllEstablishments(): Promise<Establishment[]> {
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.allEstablishments;
  await loadAppConfig();
  return cache?.allEstablishments ?? [];
}

export async function saveSiteIdentity(data: SiteIdentity, opts?: { allowEmptyName?: boolean }) {
  const { normalizeHeaderLogoRefForStorage } = await import("@/app/lib/branding-logo");
  const parsed = parseSiteIdentity(
    { ...data, assistanceEmail: PLATFORM_ASSISTANCE_EMAIL },
    opts,
  );
  const logoKey = await normalizeHeaderLogoRefForStorage(parsed.headerLogoUrl);
  if (logoKey) parsed.headerLogoUrl = logoKey;
  else delete parsed.headerLogoUrl;
  await putJson("settings/site.json", parsed);
  invalidateAppConfigCache();
}

export async function saveEstablishments(establishments: Establishment[]) {
  const parsed = establishments.map(parseEstablishment);
  await putJson("settings/establishments.json", { establishments: parsed });
  invalidateAppConfigCache();
}

export async function saveNotifications(data: NotificationsConfig) {
  const parsed = parseNotifications(data);
  await putJson("settings/notifications.json", parsed);
  invalidateAppConfigCache();
}

export async function saveStaffDirectory(rows: StaffDirectoryRow[]) {
  const parsed = parseStaffDirectoryFile({ rows });
  await putJson("settings/staff-directory.json", { rows: parsed });
  invalidateAppConfigCache();
}

export async function saveTravelsModule(data: TravelsModuleConfig) {
  const parsed = parseTravelsModule(data);
  await putJson("settings/modules/travels.json", parsed);
  invalidateAppConfigCache();
}

export async function saveProfRoomModule(data: ProfRoomModuleConfig) {
  const parsed = parseProfRoomModule(data);
  await putJson("settings/modules/prof-room.json", parsed);
  invalidateAppConfigCache();
}

export async function saveDomainPlanningModule(data: DomainPlanningModuleConfig) {
  const parsed = normalizeDomainPlanningModule(parseDomainPlanningModule(data));
  await putJson("settings/modules/domain-planning.json", parsed);
  invalidateAppConfigCache();
}

export async function saveInternatModule(data: InternatModuleConfig) {
  const parsed = parseInternatModule(data);
  await putJson("settings/modules/internat.json", parsed);
  invalidateAppConfigCache();
}

export async function saveIntegrations(data: IntegrationsConfig) {
  const parsed = parseIntegrations(data);
  await putJson("settings/integrations.json", parsed);
  invalidateAppConfigCache();
}

export async function saveExternalLinks(links: ExternalQuickLinkConfig[]) {
  const parsed = parseExternalLinksFile({ links });
  await putJson("settings/external-links.json", { links: parsed });
  invalidateAppConfigCache();
}

export async function markOnboardingComplete(step = 5) {
  const config = await loadAppConfig();
  await saveSiteIdentity({
    ...config.identity,
    onboardingCompleted: true,
    onboardingCompletedAt: new Date().toISOString(),
    onboardingStep: step,
    onboardingWizardVersion: config.identity.onboardingWizardVersion ?? 2,
  });
}

export async function saveOnboardingStep(step: number) {
  const config = await loadAppConfig();
  await saveSiteIdentity({
    ...config.identity,
    onboardingStep: step,
  });
}

export function getEstablishmentByLabel(bundle: AppConfigBundle, label: string): Establishment | null {
  return matchEstablishment(bundle.establishments, label);
}

function getEstablishmentById(bundle: AppConfigBundle, id: string): Establishment | null {
  return bundle.establishments.find((e) => e.id === id) ?? null;
}

export async function seedAppSettingsFromDefaults() {
  await saveSiteIdentity(defaultSiteIdentity(), { allowEmptyName: true });
  await saveEstablishments(defaultEstablishments());
  await saveNotifications(defaultNotifications());
  await saveStaffDirectory(defaultStaffDirectory());
  await saveTravelsModule(defaultTravelsModule());
  await saveProfRoomModule(defaultProfRoomModule());
  await saveInternatModule(defaultInternatModule());
  await saveIntegrations(defaultIntegrations());
  await saveExternalLinks(defaultExternalLinks());
}

export async function seedAppSettingsFromLaProvidence() {
  await saveSiteIdentity(laprovidenceSiteIdentity());
  await saveEstablishments(laprovidenceEstablishments());
  await saveNotifications(laprovidenceNotifications());
  await saveStaffDirectory(
    LAPROVIDENCE_STAFF_DIRECTORY.map((r) => ({
      email: r.email,
      branchId: r.branchId,
      role: r.role,
      validUntil: r.validUntil,
    })),
  );
  await saveTravelsModule(laprovidenceTravelsModule());
  await saveProfRoomModule(defaultProfRoomModule());
  await saveInternatModule(defaultInternatModule());
  await saveIntegrations(laprovidenceIntegrations());
  await saveExternalLinks(laprovidenceExternalLinks());
  await saveOrganigramConfig(laprovidenceOrganigramConfig());
}

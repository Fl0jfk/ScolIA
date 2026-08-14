import { getJson, putJson } from "@/app/lib/s3-storage";
import { DEFAULT_DOMAIN_PLANNING_DOMAINS, DEFAULT_EVARS_SESSIONS, normalizeSessionConstraint } from "@/app/lib/domain-planning-defaults";
import type {
  DomainPlanningBooking,
  DomainPlanningDomain,
  DomainPlanningSession,
  DomainPlanningSignup,
} from "@/app/lib/domain-planning-types";

const DOMAINS_KEY = "domain-planning/domains.json";
const BOOKINGS_KEY = "domain-planning/bookings.json";
const SESSIONS_KEY = "domain-planning/sessions.json";
const SIGNUPS_KEY = "domain-planning/signups.json";

const DEPRECATED_DOMAIN_IDS = new Set(["unss"]);

function parseDomain(raw: unknown): DomainPlanningDomain | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = typeof o.id === "string" ? o.id.trim() : "";
  const name = typeof o.name === "string" ? o.name.trim() : "";
  if (!id || !name) return null;
  const coordinatorClerkUserIds = Array.isArray(o.coordinatorClerkUserIds)
    ? o.coordinatorClerkUserIds.map((x) => String(x).trim()).filter(Boolean)
    : [];
  return {
    id,
    name,
    description: typeof o.description === "string" ? o.description.trim() : undefined,
    color: typeof o.color === "string" ? o.color.trim() : undefined,
    coordinatorClerkUserIds,
  };
}

export async function loadDomains(): Promise<DomainPlanningDomain[]> {
  const hit = await getJson<{ domains?: unknown[] } | unknown[]>(DOMAINS_KEY);
  const data = hit?.data;
  const raw = Array.isArray(data) ? data : (data as { domains?: unknown[] })?.domains;
  if (!raw?.length) return [...DEFAULT_DOMAIN_PLANNING_DOMAINS];
  const parsed = raw.map(parseDomain).filter(Boolean) as DomainPlanningDomain[];
  const filtered = parsed.filter((d) => !DEPRECATED_DOMAIN_IDS.has(d.id));
  return filtered.length > 0 ? filtered : [...DEFAULT_DOMAIN_PLANNING_DOMAINS];
}

export async function saveDomains(domains: DomainPlanningDomain[]): Promise<void> {
  await putJson(DOMAINS_KEY, { domains });
}

export async function loadBookings(): Promise<DomainPlanningBooking[]> {
  const hit = await getJson<DomainPlanningBooking[]>(BOOKINGS_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function saveBookings(bookings: DomainPlanningBooking[]): Promise<void> {
  await putJson(BOOKINGS_KEY, bookings);
}

export async function findDomainById(domainId: string): Promise<DomainPlanningDomain | null> {
  const domains = await loadDomains();
  return domains.find((d) => d.id === domainId) ?? null;
}

function parseSession(raw: unknown): DomainPlanningSession | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = typeof o.id === "string" ? o.id.trim() : "";
  const niveau = o.niveau;
  const seanceNumber = o.seanceNumber;
  const theme = typeof o.theme === "string" ? o.theme.trim() : "";
  const intervenantLabel = typeof o.intervenantLabel === "string" ? o.intervenantLabel.trim() : "";
  const constraint = normalizeSessionConstraint(o.intervenantConstraint, intervenantLabel);
  if (!id || !theme || !intervenantLabel) return null;
  if (niveau !== "6e" && niveau !== "5e" && niveau !== "4e" && niveau !== "3e") return null;
  if (seanceNumber !== 1 && seanceNumber !== 2 && seanceNumber !== 3) return null;
  if (!constraint) return null;
  return {
    id,
    niveau,
    seanceNumber,
    theme,
    intervenantLabel,
    intervenantConstraint: constraint,
    mixte: Boolean(o.mixte),
  };
}

export async function loadSessions(): Promise<DomainPlanningSession[]> {
  const hit = await getJson<{ sessions?: unknown[] } | unknown[]>(SESSIONS_KEY);
  const data = hit?.data;
  const raw = Array.isArray(data) ? data : (data as { sessions?: unknown[] })?.sessions;
  if (!raw?.length) return [...DEFAULT_EVARS_SESSIONS];
  const parsed = raw.map(parseSession).filter(Boolean) as DomainPlanningSession[];
  return parsed.length > 0 ? parsed : [...DEFAULT_EVARS_SESSIONS];
}

export async function saveSessions(sessions: DomainPlanningSession[]): Promise<void> {
  await putJson(SESSIONS_KEY, { sessions });
}

function parseSignup(raw: unknown): DomainPlanningSignup | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = typeof o.id === "string" ? o.id.trim() : "";
  const sessionId = typeof o.sessionId === "string" ? o.sessionId.trim() : "";
  const className = typeof o.className === "string" ? o.className.trim() : "";
  const userId = typeof o.userId === "string" ? o.userId.trim() : "";
  const firstName = typeof o.firstName === "string" ? o.firstName.trim() : "";
  const lastName = typeof o.lastName === "string" ? o.lastName.trim() : "";
  const email = typeof o.email === "string" ? o.email.trim() : "";
  const subject = typeof o.subject === "string" ? o.subject.trim() : "";
  const createdAt = typeof o.createdAt === "string" ? o.createdAt : "";
  if (!id || !sessionId || !className || !userId || !subject || !createdAt) return null;
  const validationStatus =
    o.validationStatus === "pending" ||
    o.validationStatus === "validated" ||
    o.validationStatus === "changes_requested" ||
    o.validationStatus === "rejected"
      ? o.validationStatus
      : undefined;
  return {
    id,
    sessionId,
    className,
    userId,
    firstName,
    lastName,
    email,
    subject,
    sessionIdea: typeof o.sessionIdea === "string" ? o.sessionIdea.trim() : undefined,
    createdAt,
    validationStatus,
    validatedAt: typeof o.validatedAt === "string" ? o.validatedAt : undefined,
    validatedByUserId: typeof o.validatedByUserId === "string" ? o.validatedByUserId.trim() : undefined,
    validationComment: typeof o.validationComment === "string" ? o.validationComment.trim() : undefined,
  };
}

export async function loadSignups(): Promise<DomainPlanningSignup[]> {
  const hit = await getJson<DomainPlanningSignup[]>(SIGNUPS_KEY);
  return Array.isArray(hit?.data) ? hit.data.map(parseSignup).filter(Boolean) as DomainPlanningSignup[] : [];
}

export async function saveSignups(signups: DomainPlanningSignup[]): Promise<void> {
  await putJson(SIGNUPS_KEY, signups);
}

export async function findSessionById(sessionId: string): Promise<DomainPlanningSession | null> {
  const sessions = await loadSessions();
  return sessions.find((s) => s.id === sessionId) ?? null;
}

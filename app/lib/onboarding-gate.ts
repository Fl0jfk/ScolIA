type OnboardingGateDecision =
  | { action: "allow" }
  | { action: "redirect"; path: "/onboarding" | "/configuration-en-cours" };

const ONBOARDING_EXEMPT_PREFIXES = [
  "/onboarding",
  "/configuration-en-cours",
  "/platform/setup",
  "/plateforme",
  "/sign-in",
  "/sign-up",
  "/agentIAOCR/msal-callback",
];

export function isOnboardingExemptPath(pathname: string): boolean {
  const p = pathname.split("?")[0] || "/";
  return ONBOARDING_EXEMPT_PREFIXES.some((prefix) => p === prefix || p.startsWith(`${prefix}/`));
}

export function resolveOnboardingGate(input: {
  pathname: string;
  onboardingCompleted: boolean;
  isOrgAdmin: boolean;
  isPlatformMaster?: boolean;
  reviewMode?: boolean;
}): OnboardingGateDecision {
  if (input.isPlatformMaster) return { action: "allow" };
  if (input.reviewMode && input.isOrgAdmin) return { action: "allow" };
  if (isOnboardingExemptPath(input.pathname)) return { action: "allow" };
  if (input.onboardingCompleted) return { action: "allow" };
  if (input.isOrgAdmin) return { action: "redirect", path: "/onboarding" };
  return { action: "redirect", path: "/configuration-en-cours" };
}

"use client";

import { useEffect, useMemo, useState, useTransition } from "react";
import type {
  InvitationDiploma,
  InvitationPagePublic,
  InvitationResponse,
  InvitationSituationStatus,
} from "@/app/lib/invitation-types";
import { shouldAskSituation } from "@/app/lib/invitation-types";

type Step = "eleve" | "rsvp" | "details" | "done";

type Props = {
  page: InvitationPagePublic;
};

type ConfettiPiece = {
  id: number;
  left: number;
  delay: number;
  duration: number;
  size: number;
  rotate: number;
  color: string;
  drift: number;
};

const CONFETTI_COLORS = ["#e8d48b", "#f5e6a8", "#7eb8d4", "#f0c27b", "#ffffff", "#9fd4c0"];

function buildConfetti(count: number): ConfettiPiece[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i,
    left: (i * 37 + 11) % 100,
    delay: (i % 12) * 0.12,
    duration: 2.4 + (i % 7) * 0.35,
    size: 6 + (i % 5) * 2,
    rotate: (i * 47) % 360,
    color: CONFETTI_COLORS[i % CONFETTI_COLORS.length],
    drift: ((i % 5) - 2) * 18,
  }));
}

function SparkleField({ dense = false }: { dense?: boolean }) {
  const sparks = useMemo(
    () =>
      Array.from({ length: dense ? 28 : 16 }, (_, i) => ({
        id: i,
        left: (i * 19 + 7) % 100,
        top: (i * 29 + 13) % 100,
        delay: (i % 10) * 0.35,
        size: 2 + (i % 3),
      })),
    [dense],
  );
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
      {sparks.map((s) => (
        <span
          key={s.id}
          className="inv-sparkle absolute rounded-full"
          style={{
            left: `${s.left}%`,
            top: `${s.top}%`,
            width: s.size,
            height: s.size,
            animationDelay: `${s.delay}s`,
          }}
        />
      ))}
    </div>
  );
}

function ConfettiBurst({ active }: { active: boolean }) {
  const pieces = useMemo(() => buildConfetti(42), []);
  if (!active) return null;
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
      {pieces.map((p) => (
        <span
          key={p.id}
          className="inv-confetti absolute top-[-8%]"
          style={{
            left: `${p.left}%`,
            width: p.size,
            height: p.size * 1.6,
            background: p.color,
            borderRadius: p.size > 10 ? "2px" : "50%",
            animationDelay: `${p.delay}s`,
            animationDuration: `${p.duration}s`,
            ["--inv-drift" as string]: `${p.drift}px`,
            ["--inv-rot" as string]: `${p.rotate}deg`,
          }}
        />
      ))}
    </div>
  );
}

function LaurelMark({ className }: { className?: string }) {
  return (
    <svg
      aria-hidden
      viewBox="0 0 120 48"
      className={className}
      fill="none"
    >
      <path
        d="M58 40c-10-2-22-10-28-22 8 2 16 8 22 16 2-10 8-18 18-24-4 10-8 20-12 30Z"
        fill="currentColor"
        opacity="0.85"
      />
      <path
        d="M62 40c10-2 22-10 28-22-8 2-16 8-22 16-2-10-8-18-18-24 4 10 8 20 12 30Z"
        fill="currentColor"
        opacity="0.85"
      />
      <circle cx="60" cy="14" r="3.5" fill="currentColor" />
    </svg>
  );
}

function TenantLogoMark({
  logoUrl,
  festive,
  schoolName,
}: {
  logoUrl: string | null;
  festive: boolean;
  schoolName: string;
}) {
  const [failed, setFailed] = useState(false);
  if (!logoUrl || failed) {
    return festive ? <LaurelMark className="h-10 w-28" /> : null;
  }
  return (
    // eslint-disable-next-line @next/next/no-img-element
    <img
      src={logoUrl}
      alt={schoolName || "Logo"}
      className="inv-float mx-auto h-14 w-auto max-w-[180px] object-contain"
      style={
        festive
          ? {
              filter:
                "brightness(0) saturate(100%) invert(86%) sepia(28%) saturate(700%) hue-rotate(5deg) brightness(102%) contrast(92%)",
            }
          : undefined
      }
      onError={() => setFailed(true)}
    />
  );
}

export default function InvitationPublicClient({ page }: Props) {
  const [step, setStep] = useState<Step>("eleve");
  const [eleveFirstName, setEleveFirstName] = useState("");
  const [eleveLastName, setEleveLastName] = useState("");
  const [response, setResponse] = useState<InvitationResponse | null>(null);
  const [presentCount, setPresentCount] = useState(2);
  const [parentEmail, setParentEmail] = useState("");
  const [diploma, setDiploma] = useState<InvitationDiploma | "">("");
  const [birthDate, setBirthDate] = useState("");
  const [editingExisting, setEditingExisting] = useState(false);
  const [situationStatus, setSituationStatus] = useState<InvitationSituationStatus | "">("");
  const [situationDetail, setSituationDetail] = useState("");
  const [situationEstablishment, setSituationEstablishment] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mailSent, setMailSent] = useState(false);
  const [updatedExisting, setUpdatedExisting] = useState(false);
  const [pending, startTransition] = useTransition();
  const [celebrate, setCelebrate] = useState(false);

  const whenLabel = useMemo(() => {
    if (!page.startsAt) return null;
    return new Date(page.startsAt).toLocaleString("fr-FR", {
      timeZone: "Europe/Paris",
      weekday: "long",
      day: "numeric",
      month: "long",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }, [page.startsAt]);

  const festive = page.theme !== "neutre";

  const effectiveDiploma: InvitationDiploma | null =
    page.diplomaMode === "bac"
      ? "bac"
      : page.diplomaMode === "brevet"
        ? "brevet"
        : page.diplomaMode === "both"
          ? diploma || null
          : null;

  const showSituation = shouldAskSituation(
    page.askSituation,
    effectiveDiploma,
    page.diplomaMode,
  );

  useEffect(() => {
    if (step === "done" && response === "oui" && festive) {
      setCelebrate(true);
      const t = window.setTimeout(() => setCelebrate(false), 5200);
      return () => window.clearTimeout(t);
    }
    setCelebrate(false);
    return undefined;
  }, [step, response, festive]);

  function goRsvp() {
    setError(null);
    if (!eleveFirstName.trim() || !eleveLastName.trim()) {
      setError("Indiquez le prénom et le nom de l’élève.");
      return;
    }
    if (!birthDate.trim()) {
      setError("Indiquez la date de naissance de l’élève.");
      return;
    }

    startTransition(async () => {
      try {
        const res = await fetch("/api/invitation/lookup", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            slug: page.slug,
            eleveFirstName: eleveFirstName.trim(),
            eleveLastName: eleveLastName.trim(),
            birthDate: birthDate.trim() || null,
          }),
        });
        const data = (await res.json()) as {
          ok?: boolean;
          error?: string;
          existing?: {
            response: InvitationResponse;
            presentCount: number;
            parentEmail: string;
            diploma: InvitationDiploma | null;
            situationStatus: InvitationSituationStatus | null;
            situationDetail: string;
            situationEstablishment: string;
          } | null;
        };
        if (!res.ok) throw new Error(data.error || "Vérification impossible.");
        if (data.existing) {
          setEditingExisting(true);
          setResponse(data.existing.response);
          setPresentCount(data.existing.presentCount || 2);
          setParentEmail(data.existing.parentEmail || "");
          setDiploma(data.existing.diploma || "");
          setSituationStatus(data.existing.situationStatus || "");
          setSituationDetail(data.existing.situationDetail || "");
          setSituationEstablishment(data.existing.situationEstablishment || "");
          setStep("details");
        } else {
          setEditingExisting(false);
          setStep("rsvp");
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    });
  }

  function chooseResponse(r: InvitationResponse) {
    setResponse(r);
    setError(null);
    setStep("details");
  }

  function submit() {
    setError(null);
    if (!response) {
      setError("Choisissez Oui ou Non.");
      return;
    }
    if (!parentEmail.trim()) {
      setError("Indiquez votre e-mail.");
      return;
    }
    if (page.diplomaMode === "both" && !diploma) {
      setError("Indiquez le diplôme.");
      return;
    }
    if (response === "oui") {
      if (presentCount < 1 || presentCount > page.maxPersonsPerEleve) {
        setError(`Nombre de personnes : entre 1 et ${page.maxPersonsPerEleve}.`);
        return;
      }
      if (page.placesRemaining != null && presentCount > page.placesRemaining) {
        setError(
          page.placesRemaining === 0
            ? "Il n’y a plus de places disponibles."
            : `Il ne reste que ${page.placesRemaining} place(s).`,
        );
        return;
      }
    }

    startTransition(async () => {
      try {
        const res = await fetch("/api/invitation/register", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            slug: page.slug,
            eleveFirstName: eleveFirstName.trim(),
            eleveLastName: eleveLastName.trim(),
            birthDate: birthDate.trim(),
            response,
            presentCount: response === "oui" ? presentCount : undefined,
            parentEmail: parentEmail.trim(),
            diploma:
              page.diplomaMode === "both"
                ? diploma || null
                : page.diplomaMode === "bac"
                  ? "bac"
                  : page.diplomaMode === "brevet"
                    ? "brevet"
                    : null,
            situationStatus: showSituation ? situationStatus || null : null,
            situationDetail: showSituation ? situationDetail : undefined,
            situationEstablishment: showSituation ? situationEstablishment : undefined,
            website: "",
          }),
        });
        const data = (await res.json()) as {
          success?: boolean;
          updated?: boolean;
          mailSent?: boolean;
          error?: string;
        };
        if (!res.ok) throw new Error(data.error || "Envoi impossible.");
        setMailSent(Boolean(data.mailSent));
        setUpdatedExisting(Boolean(data.updated));
        setStep("done");
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    });
  }

  const shellClass = festive
    ? "invitation-theme-diplome min-h-[100dvh]"
    : "invitation-theme-neutre min-h-[100dvh]";

  return (
    <div className={shellClass}>
      <style jsx global>{`
        .invitation-theme-diplome,
        .invitation-theme-neutre {
          font-family: Figtree, "Segoe UI", sans-serif;
        }
        .invitation-theme-diplome .inv-display {
          font-family: "Cormorant Garamond", "Palatino Linotype", Palatino, serif;
        }
        .invitation-theme-diplome {
          --inv-ink: #f4f7fb;
          --inv-muted: #b7c3d6;
          --inv-accent: #e8d48b;
          --inv-accent-deep: #c9a227;
          --inv-accent-soft: rgba(232, 212, 139, 0.14);
          --inv-panel: rgba(12, 24, 48, 0.78);
          --inv-border: rgba(232, 212, 139, 0.28);
          --inv-input-bg: rgba(255, 255, 255, 0.06);
          --inv-input-ink: #f4f7fb;
          color: var(--inv-ink);
          background:
            radial-gradient(ellipse 90% 55% at 50% -10%, rgba(232, 212, 139, 0.22) 0%, transparent 55%),
            radial-gradient(ellipse 50% 40% at 0% 80%, rgba(70, 120, 180, 0.25) 0%, transparent 60%),
            radial-gradient(ellipse 45% 35% at 100% 70%, rgba(40, 90, 140, 0.28) 0%, transparent 55%),
            linear-gradient(165deg, #071022 0%, #0f1f3a 48%, #152848 100%);
        }
        .invitation-theme-neutre {
          --inv-ink: #0f172a;
          --inv-muted: #64748b;
          --inv-accent: #0f766e;
          --inv-accent-deep: #0f766e;
          --inv-accent-soft: #ccfbf1;
          --inv-panel: rgba(255, 255, 255, 0.95);
          --inv-border: rgba(15, 23, 42, 0.1);
          --inv-input-bg: #ffffff;
          --inv-input-ink: #0f172a;
          background: linear-gradient(180deg, #f8fafc 0%, #eef2ff 100%);
        }
        @keyframes inv-rise {
          from {
            opacity: 0;
            transform: translateY(16px) scale(0.985);
          }
          to {
            opacity: 1;
            transform: translateY(0) scale(1);
          }
        }
        @keyframes inv-shimmer {
          0%,
          100% {
            opacity: 0.35;
            transform: scale(1);
          }
          50% {
            opacity: 0.75;
            transform: scale(1.08);
          }
        }
        @keyframes inv-sparkle {
          0%,
          100% {
            opacity: 0.15;
            transform: scale(0.7);
          }
          50% {
            opacity: 0.95;
            transform: scale(1.25);
          }
        }
        @keyframes inv-float {
          0%,
          100% {
            transform: translateY(0);
          }
          50% {
            transform: translateY(-6px);
          }
        }
        @keyframes inv-confetti-fall {
          0% {
            opacity: 0;
            transform: translate3d(0, -10%, 0) rotate(0deg);
          }
          12% {
            opacity: 1;
          }
          100% {
            opacity: 0;
            transform: translate3d(var(--inv-drift, 20px), 110vh, 0) rotate(var(--inv-rot, 240deg));
          }
        }
        @keyframes inv-ribbon {
          0%,
          100% {
            transform: rotate(-8deg);
          }
          50% {
            transform: rotate(8deg);
          }
        }
        .inv-rise {
          animation: inv-rise 0.65s cubic-bezier(0.22, 1, 0.36, 1) both;
        }
        .inv-rise-delay {
          animation: inv-rise 0.7s cubic-bezier(0.22, 1, 0.36, 1) 0.1s both;
        }
        .inv-glow {
          animation: inv-shimmer 5s ease-in-out infinite;
        }
        .inv-sparkle {
          background: #e8d48b;
          box-shadow: 0 0 6px rgba(232, 212, 139, 0.8);
          animation: inv-sparkle 3.2s ease-in-out infinite;
        }
        .inv-float {
          animation: inv-float 4.5s ease-in-out infinite;
        }
        .inv-confetti {
          animation-name: inv-confetti-fall;
          animation-timing-function: cubic-bezier(0.25, 0.7, 0.35, 1);
          animation-fill-mode: both;
        }
        .inv-ribbon {
          animation: inv-ribbon 3.8s ease-in-out infinite;
          transform-origin: 50% 0%;
        }
        .invitation-theme-diplome input,
        .invitation-theme-diplome select,
        .invitation-theme-diplome textarea {
          background: var(--inv-input-bg);
          color: var(--inv-input-ink);
        }
        .invitation-theme-diplome option {
          color: #0f172a;
        }
      `}</style>

      <div className="relative mx-auto max-w-lg px-4 py-10 sm:py-16">
        {festive ? <SparkleField dense /> : null}
        <ConfettiBurst active={celebrate} />

        <div
          className="inv-rise relative overflow-hidden rounded-[1.85rem] border p-6 sm:p-8 shadow-2xl shadow-black/30 backdrop-blur-md"
          style={{
            background: "var(--inv-panel)",
            borderColor: "var(--inv-border)",
            color: "var(--inv-ink)",
          }}
        >
          {festive ? (
            <>
              <div
                aria-hidden
                className="inv-glow pointer-events-none absolute -right-10 -top-12 h-44 w-44 rounded-full"
                style={{
                  background:
                    "radial-gradient(circle, rgba(232,212,139,0.35) 0%, transparent 70%)",
                }}
              />
              <div
                aria-hidden
                className="inv-glow pointer-events-none absolute -bottom-16 -left-10 h-40 w-40 rounded-full"
                style={{
                  background:
                    "radial-gradient(circle, rgba(90,140,200,0.28) 0%, transparent 70%)",
                  animationDelay: "1.2s",
                }}
              />
              <div className="mb-3 flex justify-center text-[var(--inv-accent)]">
                <TenantLogoMark
                  logoUrl={page.logoUrl}
                  festive={festive}
                  schoolName={page.schoolName}
                />
              </div>
            </>
          ) : page.logoUrl ? (
            <div className="mb-3 flex justify-center">
              <TenantLogoMark
                logoUrl={page.logoUrl}
                festive={false}
                schoolName={page.schoolName}
              />
            </div>
          ) : null}

          <p
            className="text-center text-[11px] font-semibold uppercase tracking-[0.28em]"
            style={{ color: "var(--inv-muted)" }}
          >
            {page.schoolName || "Invitation"}
          </p>
          <h1
            className="inv-display mt-2 text-center text-3xl sm:text-4xl font-semibold tracking-tight leading-[1.12]"
          >
            {page.title}
          </h1>

          {festive ? (
            <div className="mx-auto mt-3 flex justify-center">
              <span
                aria-hidden
                className="inv-ribbon inline-block h-1.5 w-16 rounded-full"
                style={{
                  background:
                    "linear-gradient(90deg, transparent, var(--inv-accent), transparent)",
                }}
              />
            </div>
          ) : null}

          {page.intro ? (
            <p
              className="mt-4 text-center text-sm leading-relaxed"
              style={{ color: "var(--inv-muted)" }}
            >
              {page.intro}
            </p>
          ) : null}

          {(whenLabel || page.location) && (
            <div
              className="mt-5 space-y-1 rounded-2xl px-4 py-3 text-center text-sm"
              style={{ background: "var(--inv-accent-soft)", color: "var(--inv-ink)" }}
            >
              {whenLabel ? <p className="font-semibold capitalize">{whenLabel}</p> : null}
              {page.location ? (
                <p style={{ color: "var(--inv-muted)" }}>{page.location}</p>
              ) : null}
            </div>
          )}

          {error ? (
            <p className="mt-4 rounded-xl border border-rose-300/50 bg-rose-500/15 px-3 py-2 text-sm text-rose-100">
              {error}
            </p>
          ) : null}

          <div className="inv-rise-delay mt-6 space-y-5">
            {!page.rsvpOpen ? (
              <p className="rounded-xl border border-amber-300/40 bg-amber-500/10 px-3 py-3 text-center text-sm">
                Les inscriptions sont closes
                {page.rsvpClosesAt
                  ? ` (limite : ${new Date(page.rsvpClosesAt).toLocaleString("fr-FR", {
                      timeZone: "Europe/Paris",
                    })})`
                  : ""}
                .
              </p>
            ) : null}

            {page.rsvpOpen && step === "eleve" ? (
              <>
                {page.requireEligible ? (
                  <p className="text-center text-xs" style={{ color: "var(--inv-muted)" }}>
                    Réservé aux élèves de la liste. En cas de petite faute d’orthographe, 2 critères
                    sur 3 (prénom, nom, date de naissance) suffisent.
                  </p>
                ) : null}
                <div className="grid gap-3 sm:grid-cols-2">
                  <label
                    className="flex flex-col gap-1 text-xs font-semibold"
                    style={{ color: "var(--inv-muted)" }}
                  >
                    Prénom de l’élève
                    <input
                      value={eleveFirstName}
                      onChange={(e) => setEleveFirstName(e.target.value)}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                      autoComplete="given-name"
                      required
                    />
                  </label>
                  <label
                    className="flex flex-col gap-1 text-xs font-semibold"
                    style={{ color: "var(--inv-muted)" }}
                  >
                    Nom de l’élève
                    <input
                      value={eleveLastName}
                      onChange={(e) => setEleveLastName(e.target.value)}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                      autoComplete="family-name"
                      required
                    />
                  </label>
                </div>
                <label
                  className="flex flex-col gap-1 text-xs font-semibold"
                  style={{ color: "var(--inv-muted)" }}
                >
                  Date de naissance de l’élève
                  <input
                    type="date"
                    value={birthDate}
                    onChange={(e) => setBirthDate(e.target.value)}
                    className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                    style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                    required
                  />
                </label>
                <button
                  type="button"
                  onClick={goRsvp}
                  disabled={pending}
                  className="w-full rounded-2xl px-4 py-3 text-sm font-bold transition hover:brightness-110 disabled:opacity-60"
                  style={{
                    background: festive
                      ? "linear-gradient(135deg, var(--inv-accent) 0%, var(--inv-accent-deep) 100%)"
                      : "var(--inv-accent)",
                    color: festive ? "#0c1b33" : "#ffffff",
                  }}
                >
                  {pending ? "Vérification…" : "Continuer"}
                </button>
              </>
            ) : null}

            {page.rsvpOpen && step === "rsvp" ? (
              <>
                <p className="text-center text-sm font-semibold">
                  {eleveFirstName} {eleveLastName} — serez-vous présents ?
                </p>
                <div className="grid gap-3 sm:grid-cols-2">
                  <button
                    type="button"
                    onClick={() => chooseResponse("oui")}
                    className="rounded-2xl border-2 px-4 py-4 text-sm font-bold transition hover:scale-[1.02]"
                    style={{
                      borderColor: "var(--inv-accent)",
                      color: festive ? "var(--inv-accent)" : "var(--inv-accent)",
                      background: festive ? "rgba(232,212,139,0.08)" : "transparent",
                    }}
                  >
                    Oui, je viens
                  </button>
                  <button
                    type="button"
                    onClick={() => chooseResponse("non")}
                    className="rounded-2xl border px-4 py-4 text-sm font-bold transition hover:bg-white/5"
                    style={{ borderColor: "var(--inv-border)", color: "var(--inv-muted)" }}
                  >
                    Non, je ne viens pas
                  </button>
                </div>
                <button
                  type="button"
                  onClick={() => setStep("eleve")}
                  className="mx-auto block text-xs font-semibold underline"
                  style={{ color: "var(--inv-muted)" }}
                >
                  Modifier l’élève
                </button>
              </>
            ) : null}

            {page.rsvpOpen && step === "details" && response ? (
              <>
                <p className="text-center text-sm font-semibold">
                  {editingExisting
                    ? "Vous avez déjà répondu — vous pouvez modifier"
                    : response === "oui"
                      ? "Compléter votre inscription"
                      : "Confirmer votre absence"}
                </p>

                {response === "oui" ? (
                  <label
                    className="flex flex-col gap-1 text-xs font-semibold"
                    style={{ color: "var(--inv-muted)" }}
                  >
                    Nombre de personnes présentes (moi compris)
                    <input
                      type="number"
                      min={1}
                      max={page.maxPersonsPerEleve}
                      value={presentCount}
                      onChange={(e) => setPresentCount(Number(e.target.value) || 1)}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                    />
                    <span className="font-medium">
                      Maximum {page.maxPersonsPerEleve}
                      {page.placesRemaining != null
                        ? ` · ${page.placesRemaining} place(s) restante(s)`
                        : ""}
                    </span>
                  </label>
                ) : null}

                {page.diplomaMode === "both" ? (
                  <label
                    className="flex flex-col gap-1 text-xs font-semibold"
                    style={{ color: "var(--inv-muted)" }}
                  >
                    Diplôme
                    <select
                      value={diploma}
                      onChange={(e) => setDiploma(e.target.value as InvitationDiploma | "")}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                    >
                      <option value="">— Choisir —</option>
                      <option value="bac">Baccalauréat</option>
                      <option value="brevet">Brevet</option>
                    </select>
                  </label>
                ) : null}

                {showSituation ? (
                  <div className="space-y-3 rounded-2xl border px-3 py-3" style={{ borderColor: "var(--inv-border)" }}>
                    <p className="text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                      Situation actuelle <span className="font-medium opacity-70">(optionnel)</span>
                    </p>
                    <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                      Statut
                      <select
                        value={situationStatus}
                        onChange={(e) =>
                          setSituationStatus(e.target.value as InvitationSituationStatus | "")
                        }
                        className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                        style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                      >
                        <option value="">— Ne pas préciser —</option>
                        <option value="etudes">Études / poursuite d’études</option>
                        <option value="emploi">Emploi / stage</option>
                        <option value="autre">Autre</option>
                      </select>
                    </label>
                    <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                      Filière / précision
                      <input
                        value={situationDetail}
                        onChange={(e) => setSituationDetail(e.target.value)}
                        placeholder="Ex. Licence LEA, BTS…"
                        className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                        style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                      />
                    </label>
                    <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                      Établissement / structure
                      <input
                        value={situationEstablishment}
                        onChange={(e) => setSituationEstablishment(e.target.value)}
                        placeholder="Ex. Université de Caen"
                        className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                        style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                      />
                    </label>
                  </div>
                ) : null}

                <label
                  className="flex flex-col gap-1 text-xs font-semibold"
                  style={{ color: "var(--inv-muted)" }}
                >
                  Votre e-mail
                  <input
                    type="email"
                    value={parentEmail}
                    onChange={(e) => setParentEmail(e.target.value)}
                    className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                    style={{ borderColor: "var(--inv-border)", color: "var(--inv-input-ink)" }}
                    autoComplete="email"
                  />
                </label>

                <div className="flex flex-col gap-2">
                  <button
                    type="button"
                    disabled={pending}
                    onClick={submit}
                    className="w-full rounded-2xl px-4 py-3 text-sm font-bold transition hover:brightness-110 disabled:opacity-60"
                    style={{
                      background: festive
                        ? "linear-gradient(135deg, var(--inv-accent) 0%, var(--inv-accent-deep) 100%)"
                        : "var(--inv-accent)",
                      color: festive ? "#0c1b33" : "#ffffff",
                    }}
                  >
                    {pending
                      ? "Envoi…"
                      : editingExisting
                        ? "Mettre à jour ma réponse"
                        : "Valider"}
                  </button>
                  <button
                    type="button"
                    onClick={() => setStep(editingExisting ? "eleve" : "rsvp")}
                    className="text-xs font-semibold underline"
                    style={{ color: "var(--inv-muted)" }}
                  >
                    Retour
                  </button>
                </div>
              </>
            ) : null}

            {step === "done" ? (
              <div className="space-y-3 py-2 text-center">
                {festive && response === "oui" ? (
                  <p
                    aria-hidden
                    className="inv-float text-3xl"
                    style={{ color: "var(--inv-accent)" }}
                  >
                    ★
                  </p>
                ) : null}
                <p className={`inv-display text-2xl font-semibold`}>
                  {festive && response === "oui"
                    ? "À très bientôt !"
                    : updatedExisting
                      ? "Réponse mise à jour"
                      : "Merci"}
                </p>
                <p className="text-sm leading-relaxed" style={{ color: "var(--inv-muted)" }}>
                  {updatedExisting ? (
                    <>
                      Votre inscription pour{" "}
                      <strong style={{ color: "var(--inv-ink)" }}>
                        {eleveFirstName} {eleveLastName}
                      </strong>{" "}
                      a bien été modifiée
                      {response === "oui"
                        ? ` (${presentCount} personne${presentCount > 1 ? "s" : ""}).`
                        : " (absence confirmée)."}
                      {mailSent ? " Un e-mail de confirmation vous a été envoyé." : ""}
                    </>
                  ) : response === "oui" ? (
                    <>
                      Présence confirmée pour{" "}
                      <strong style={{ color: "var(--inv-ink)" }}>
                        {eleveFirstName} {eleveLastName}
                      </strong>{" "}
                      ({presentCount} personne{presentCount > 1 ? "s" : ""}).
                      {mailSent
                        ? " Un e-mail de confirmation avec le calendrier (.ics) vous a été envoyé."
                        : " Votre réponse est enregistrée."}
                    </>
                  ) : (
                    <>
                      Nous avons bien noté que vous ne pourrez pas venir pour{" "}
                      <strong style={{ color: "var(--inv-ink)" }}>
                        {eleveFirstName} {eleveLastName}
                      </strong>
                      .
                      {mailSent ? " Un e-mail de confirmation vous a été envoyé." : ""}
                    </>
                  )}
                </p>
              </div>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}

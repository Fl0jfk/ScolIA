"use client";

import { useMemo, useState, useTransition } from "react";
import type { InvitationDiploma, InvitationPagePublic, InvitationResponse } from "@/app/lib/invitation-types";

type Step = "eleve" | "rsvp" | "details" | "done";

type Props = {
  page: InvitationPagePublic;
};

export default function InvitationPublicClient({ page }: Props) {
  const [step, setStep] = useState<Step>("eleve");
  const [eleveFirstName, setEleveFirstName] = useState("");
  const [eleveLastName, setEleveLastName] = useState("");
  const [response, setResponse] = useState<InvitationResponse | null>(null);
  const [presentCount, setPresentCount] = useState(2);
  const [parentEmail, setParentEmail] = useState("");
  const [diploma, setDiploma] = useState<InvitationDiploma | "">("");
  const [error, setError] = useState<string | null>(null);
  const [mailSent, setMailSent] = useState(false);
  const [pending, startTransition] = useTransition();

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

  const theme = page.theme === "neutre" ? "neutre" : "remise_diplome";

  function goRsvp() {
    setError(null);
    if (!eleveFirstName.trim() || !eleveLastName.trim()) {
      setError("Indiquez le prénom et le nom de l’élève.");
      return;
    }
    setStep("rsvp");
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
            website: "",
          }),
        });
        const data = (await res.json()) as {
          success?: boolean;
          mailSent?: boolean;
          error?: string;
        };
        if (!res.ok) throw new Error(data.error || "Envoi impossible.");
        setMailSent(Boolean(data.mailSent));
        setStep("done");
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    });
  }

  const shellClass =
    theme === "remise_diplome"
      ? "invitation-theme-diplome min-h-[70vh]"
      : "invitation-theme-neutre min-h-[70vh]";

  return (
    <div className={shellClass}>
      <style jsx global>{`
        .invitation-theme-diplome {
          --inv-ink: #1a1528;
          --inv-muted: #5c5470;
          --inv-accent: #c45c26;
          --inv-accent-soft: #f3e0d2;
          --inv-panel: rgba(255, 252, 247, 0.92);
          --inv-border: rgba(26, 21, 40, 0.12);
          background:
            radial-gradient(ellipse 80% 50% at 10% 0%, #f6d9b8 0%, transparent 55%),
            radial-gradient(ellipse 70% 45% at 90% 10%, #d8c4f0 0%, transparent 50%),
            linear-gradient(165deg, #faf6f0 0%, #efe8df 45%, #e8e2f2 100%);
        }
        .invitation-theme-neutre {
          --inv-ink: #0f172a;
          --inv-muted: #64748b;
          --inv-accent: #0f766e;
          --inv-accent-soft: #ccfbf1;
          --inv-panel: rgba(255, 255, 255, 0.95);
          --inv-border: rgba(15, 23, 42, 0.1);
          background: linear-gradient(180deg, #f8fafc 0%, #eef2ff 100%);
        }
        @keyframes inv-rise {
          from {
            opacity: 0;
            transform: translateY(12px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }
        @keyframes inv-glow {
          0%,
          100% {
            opacity: 0.55;
          }
          50% {
            opacity: 0.9;
          }
        }
        .inv-rise {
          animation: inv-rise 0.55s ease-out both;
        }
        .inv-rise-delay {
          animation: inv-rise 0.65s ease-out 0.08s both;
        }
        .inv-ornament {
          animation: inv-glow 4s ease-in-out infinite;
        }
      `}</style>

      <div className="mx-auto max-w-lg px-4 py-10 sm:py-14">
        <div className="inv-rise relative overflow-hidden rounded-[1.75rem] border p-6 sm:p-8 shadow-xl shadow-black/5"
          style={{ background: "var(--inv-panel)", borderColor: "var(--inv-border)", color: "var(--inv-ink)" }}
        >
          {theme === "remise_diplome" ? (
            <div
              aria-hidden
              className="inv-ornament pointer-events-none absolute -right-8 -top-8 h-36 w-36 rounded-full"
              style={{
                background:
                  "radial-gradient(circle, rgba(196,92,38,0.35) 0%, transparent 70%)",
              }}
            />
          ) : null}

          <p
            className="text-[11px] font-semibold uppercase tracking-[0.22em]"
            style={{ color: "var(--inv-muted)" }}
          >
            {page.schoolName || "Invitation"}
          </p>
          <h1
            className="mt-2 text-3xl sm:text-4xl font-semibold tracking-tight leading-[1.1]"
            style={{
              fontFamily:
                theme === "remise_diplome"
                  ? '"Cormorant Garamond", "Palatino Linotype", Palatino, serif'
                  : "inherit",
            }}
          >
            {page.title}
          </h1>
          {page.intro ? (
            <p className="mt-3 text-sm leading-relaxed" style={{ color: "var(--inv-muted)" }}>
              {page.intro}
            </p>
          ) : null}
          {(whenLabel || page.location) && (
            <div
              className="mt-4 space-y-1 rounded-2xl px-4 py-3 text-sm"
              style={{ background: "var(--inv-accent-soft)", color: "var(--inv-ink)" }}
            >
              {whenLabel ? <p className="font-semibold capitalize">{whenLabel}</p> : null}
              {page.location ? <p style={{ color: "var(--inv-muted)" }}>{page.location}</p> : null}
            </div>
          )}

          {error ? (
            <p className="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
              {error}
            </p>
          ) : null}

          <div className="inv-rise-delay mt-6 space-y-5">
            {step === "eleve" ? (
              <>
                <p className="text-sm font-semibold">Élève concerné</p>
                <div className="grid gap-3 sm:grid-cols-2">
                  <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                    Prénom
                    <input
                      value={eleveFirstName}
                      onChange={(e) => setEleveFirstName(e.target.value)}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-ink)" }}
                      autoComplete="given-name"
                    />
                  </label>
                  <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                    Nom
                    <input
                      value={eleveLastName}
                      onChange={(e) => setEleveLastName(e.target.value)}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-ink)" }}
                      autoComplete="family-name"
                    />
                  </label>
                </div>
                <button
                  type="button"
                  onClick={goRsvp}
                  className="w-full rounded-2xl px-4 py-3 text-sm font-bold text-white transition hover:opacity-95"
                  style={{ background: "var(--inv-accent)" }}
                >
                  Continuer
                </button>
              </>
            ) : null}

            {step === "rsvp" ? (
              <>
                <p className="text-sm font-semibold">
                  {eleveFirstName} {eleveLastName} — serez-vous présents ?
                </p>
                <div className="grid gap-3 sm:grid-cols-2">
                  <button
                    type="button"
                    onClick={() => chooseResponse("oui")}
                    className="rounded-2xl border-2 px-4 py-4 text-sm font-bold transition hover:scale-[1.01]"
                    style={{ borderColor: "var(--inv-accent)", color: "var(--inv-accent)" }}
                  >
                    Oui, je viens
                  </button>
                  <button
                    type="button"
                    onClick={() => chooseResponse("non")}
                    className="rounded-2xl border px-4 py-4 text-sm font-bold transition hover:bg-black/[0.03]"
                    style={{ borderColor: "var(--inv-border)", color: "var(--inv-muted)" }}
                  >
                    Non, je ne viens pas
                  </button>
                </div>
                <button
                  type="button"
                  onClick={() => setStep("eleve")}
                  className="text-xs font-semibold underline"
                  style={{ color: "var(--inv-muted)" }}
                >
                  Modifier l’élève
                </button>
              </>
            ) : null}

            {step === "details" && response ? (
              <>
                <p className="text-sm font-semibold">
                  {response === "oui" ? "Compléter votre inscription" : "Confirmer votre absence"}
                </p>

                {response === "oui" ? (
                  <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                    Nombre de personnes présentes (moi compris)
                    <input
                      type="number"
                      min={1}
                      max={page.maxPersonsPerEleve}
                      value={presentCount}
                      onChange={(e) => setPresentCount(Number(e.target.value) || 1)}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-ink)" }}
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
                  <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                    Diplôme
                    <select
                      value={diploma}
                      onChange={(e) => setDiploma(e.target.value as InvitationDiploma | "")}
                      className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                      style={{ borderColor: "var(--inv-border)", color: "var(--inv-ink)" }}
                    >
                      <option value="">— Choisir —</option>
                      <option value="bac">Baccalauréat</option>
                      <option value="brevet">Brevet</option>
                    </select>
                  </label>
                ) : null}

                <label className="flex flex-col gap-1 text-xs font-semibold" style={{ color: "var(--inv-muted)" }}>
                  Votre e-mail
                  <input
                    type="email"
                    value={parentEmail}
                    onChange={(e) => setParentEmail(e.target.value)}
                    className="rounded-xl border px-3 py-2.5 text-sm font-medium"
                    style={{ borderColor: "var(--inv-border)", color: "var(--inv-ink)" }}
                    autoComplete="email"
                  />
                </label>

                <div className="flex flex-col gap-2">
                  <button
                    type="button"
                    disabled={pending}
                    onClick={submit}
                    className="w-full rounded-2xl px-4 py-3 text-sm font-bold text-white transition hover:opacity-95 disabled:opacity-60"
                    style={{ background: "var(--inv-accent)" }}
                  >
                    {pending ? "Envoi…" : "Valider"}
                  </button>
                  <button
                    type="button"
                    onClick={() => setStep("rsvp")}
                    className="text-xs font-semibold underline"
                    style={{ color: "var(--inv-muted)" }}
                  >
                    Retour
                  </button>
                </div>
              </>
            ) : null}

            {step === "done" ? (
              <div className="space-y-3 text-center py-2">
                <p
                  className="text-2xl font-semibold"
                  style={{
                    fontFamily:
                      theme === "remise_diplome"
                        ? '"Cormorant Garamond", "Palatino Linotype", Palatino, serif'
                        : "inherit",
                  }}
                >
                  Merci
                </p>
                <p className="text-sm leading-relaxed" style={{ color: "var(--inv-muted)" }}>
                  {response === "oui" ? (
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

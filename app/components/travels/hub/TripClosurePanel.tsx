"use client";

/**
 * Clôture des inscriptions (circuit v2) : état de la clôture, liste officielle, bon cuisine jour par jour
 * (l'organisateur n'ajoute que les repas adultes), devis rectifié, désistements, ZIP des engagements (compta).
 * La clôture, l'envoi de la commande cuisine et le désistement demandent toujours une confirmation humaine.
 */

import { useCallback, useEffect, useState } from "react";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import { dateFr } from "@/app/lib/travels-family-page";
import type { ClosureView } from "@/app/lib/travels-v2-lot4-store";

type Payload = ClosureView & { canWrite: boolean; canZip: boolean; isDirection: boolean };

export function TripClosurePanel({ tripId, userName }: { tripId: string; userName?: string }) {
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [adults, setAdults] = useState<Record<string, string>>({});
  const [withdrawIne, setWithdrawIne] = useState("");
  const [withdrawNote, setWithdrawNote] = useState("");

  const load = useCallback(async () => {
    try {
      const res = await fetch(`/api/travels/v2/closure?tripId=${encodeURIComponent(tripId)}`, { cache: "no-store" });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      const p = j as Payload;
      setData(p);
      setAdults(Object.fromEntries((p.kitchen?.days || []).map((d) => [d.key, String(d.adults)])));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible");
    }
  }, [tripId]);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (body: Record<string, unknown>, okMsg?: string): Promise<{ ok: boolean; json: Record<string, unknown> }> => {
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/travels/v2/closure", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, ...body }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Action impossible");
      if (okMsg) setMsg(okMsg);
      await load();
      return { ok: true, json: j };
    } catch (e) {
      setError(e instanceof Error ? e.message : "Action impossible");
      return { ok: false, json: {} };
    } finally {
      setBusy(false);
    }
  };

  if (error && !data) return <TripAlert tone="warning" title="Clôture indisponible">{error}</TripAlert>;
  if (!data) return null;
  if (data.status !== "INSCRIPTIONS_OUVERTES" && !data.closed) return null;

  const adultsPayload = () =>
    Object.fromEntries(Object.entries(adults).map(([k, v]) => [k, Math.max(0, Math.floor(Number(v) || 0))]));

  const close = async () => {
    if (!data.open) return;
    const txt =
      "Clôturer les inscriptions ?\n\nLa liste officielle sera figée (familles ayant signé « oui »), le bon cuisine calculé, " +
      "la liste envoyée au transporteur et les engagements transmis à la comptabilité (ce clic vaut confirmation de l'envoi). Les familles ne pourront plus répondre.";
    if (!window.confirm(txt)) return;
    await post({ action: "close", confirm: true }, "Inscriptions clôturées.");
  };

  const confirmSend = async () => {
    const txt =
      "Confirmer et envoyer ?\n\nLa liste nominative des élèves part au transporteur (si le voyage a un bus) et la comptabilité est prévenue " +
      "par e-mail. Vérifiez d'abord la liste officielle ci-dessous.";
    if (!window.confirm(txt)) return;
    await post({ action: "confirm_send", confirm: true }, "Envoi confirmé : la liste part au transporteur et la comptabilité est prévenue.");
  };

  const sendCuisine = async (send: boolean) => {
    if (
      send &&
      !window.confirm(
        data.kitchen?.sentAt
          ? "Renvoyer la commande au chef (annule et remplace la précédente) ?"
          : "Envoyer la commande au chef avec ces repas (élèves + adultes) ?",
      )
    ) {
      return;
    }
    await post({ action: "adult_meals", adults: adultsPayload(), send }, send ? "Commande cuisine envoyée au chef." : "Repas adultes enregistrés.");
  };

  const rectified = async () => {
    if (!window.confirm("Envoyer une demande de devis rectifié (avenant effectif) au transporteur ?")) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/travels/send-transport-amendment", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, userName: userName || "La Providence" }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Envoi impossible");
      setMsg("Demande de devis rectifié envoyée.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Envoi impossible");
    } finally {
      setBusy(false);
    }
  };

  const withdraw = async () => {
    const o = data.official.find((x) => x.ine === withdrawIne);
    if (!o) return;
    const txt =
      `Noter le désistement de ${o.studentName} ?\n\nL'élève sort de la liste ; la cuisine (si paniers) et la comptabilité sont prévenues. ` +
      "L'engagement reste archivé avec la mention « désisté ».";
    if (!window.confirm(txt)) return;
    const r = await post({ action: "withdraw", ine: withdrawIne, note: withdrawNote.trim() || undefined, confirm: true }, "Désistement enregistré.");
    if (r.ok) {
      setWithdrawIne("");
      setWithdrawNote("");
    }
  };

  const zip = async () => {
    const r = await post({ action: "zip_link" });
    if (r.ok && typeof r.json.url === "string") window.open(r.json.url, "_blank", "noopener");
  };

  return (
    <TripSection
      title="Clôture des inscriptions"
      subtitle="Liste officielle, bon cuisine, transporteur et comptabilité"
      icon="🔒"
      accent="emerald"
      action={
        data.canZip && data.closed ? (
          <TripButton variant="secondary" size="sm" onClick={() => void zip()} disabled={busy}>
            Télécharger le ZIP des engagements
          </TripButton>
        ) : undefined
      }
    >
      <div className="space-y-4">
        {error && (
          <TripAlert tone="warning" title="Erreur">
            {error}
          </TripAlert>
        )}
        {msg && (
          <TripAlert tone="success" title={msg}>
            {null}
          </TripAlert>
        )}

        {data.open && (
          <div className="space-y-2">
            <p className="text-sm text-slate-700">
              Clôture automatique prévue {data.open.dueDate ? `après le ${dateFr(data.open.dueDate)}` : "à la date limite"} (J-14 au plus tard).{" "}
              {data.open.allAnswered
                ? "Toutes les familles ont répondu : vous pouvez clôturer dès maintenant."
                : `${data.open.unansweredCount} famille(s) n'ont pas encore répondu.`}
            </p>
            {data.open.suspended > 0 && (
              <TripAlert tone="warning" title="Clôture bloquée">
                {data.open.suspended} élève(s) en suspens (désaccord entre responsables) : la direction doit trancher dans la vue Inscriptions.
              </TripAlert>
            )}
            {data.canWrite && (
              <TripButton onClick={() => void close()} disabled={busy || !data.open.canClose}>
                Clôturer les inscriptions
              </TripButton>
            )}
            {!data.open.canClose && data.open.cannotCloseReason && <p className="text-xs text-slate-500">{data.open.cannotCloseReason}</p>}
          </div>
        )}

        {data.closure && (
          <>
            <p className="text-sm text-slate-700">
              Clôturé le {new Date(data.closure.closedAt).toLocaleDateString("fr-FR")} (
              {data.closure.mode === "auto" ? "automatiquement" : "confirmé par l'équipe"}) — <strong>{data.closure.officialCount} élève(s)</strong> dans la
              liste officielle.
              {data.needsBus && (data.closure.transportListSentAt ? " Liste envoyée au transporteur." : " Liste transporteur : pas encore envoyée.")}{" "}
              {data.closure.comptaMailSentAt ? "Comptabilité prévenue." : "Comptabilité : e-mail pas encore parti."}
            </p>
            {data.awaitingSendConfirmation && !data.purged && (
              <TripAlert tone="warning" title="Liste figée — envoi à confirmer">
                La clôture automatique a figé la liste officielle, mais rien n&apos;est encore parti à l&apos;extérieur. Vérifiez la liste puis confirmez :
                elle part alors au transporteur et la comptabilité est prévenue. Les engagements signés (ZIP) sont déjà téléchargeables par la comptabilité.
                {data.canWrite && (
                  <div className="mt-2">
                    <TripButton variant="warning" size="sm" onClick={() => void confirmSend()} disabled={busy}>
                      Confirmer et envoyer
                    </TripButton>
                  </div>
                )}
              </TripAlert>
            )}
            {data.purged ? (
              <TripAlert tone="muted" title="Voyage terminé">
                Les pièces des familles ont été supprimées ; seuls les engagements sont conservés.
              </TripAlert>
            ) : data.purgeOn ? (
              <p className="text-xs text-slate-500">
                Le {dateFr(data.purgeOn)} (lendemain du retour), toutes les pièces et réponses pratiques des familles seront supprimées ; seuls les
                engagements sont conservés.
              </p>
            ) : null}

            {data.closure.rectifiedQuote?.proposed && (
              <TripAlert tone="warning" title="Devis rectifié conseillé">
                {data.closure.rectifiedQuote.reason}
                {data.canWrite && !data.purged && (
                  <div className="mt-2">
                    <TripButton variant="warning" size="sm" onClick={() => void rectified()} disabled={busy}>
                      Demander un devis rectifié
                    </TripButton>
                  </div>
                )}
              </TripAlert>
            )}
            {data.closure.rectifiedQuote && !data.closure.rectifiedQuote.proposed && data.needsBus && (
              <p className="text-xs text-slate-500">Devis transport : {data.closure.rectifiedQuote.reason}</p>
            )}
          </>
        )}

        {data.kitchen && (
          <div className="space-y-2">
            <p className="text-sm font-bold">Bon cuisine (calculé à partir des réponses des familles)</p>
            <div className="overflow-x-auto rounded-xl border border-slate-200">
              <table className="w-full text-xs border-collapse min-w-[520px]">
                <thead>
                  <tr className="bg-emerald-600 text-white">
                    <th className="text-left p-2">Jour</th>
                    <th className="p-2">Élèves</th>
                    <th className="p-2">Classique</th>
                    <th className="p-2">Sans porc</th>
                    <th className="p-2">Végétarien</th>
                    <th className="p-2">Repas adultes</th>
                    <th className="p-2">Total</th>
                  </tr>
                </thead>
                <tbody>
                  {data.kitchen.days.map((d) => (
                    <tr key={d.key} className="odd:bg-white even:bg-slate-50">
                      <td className="p-2 font-medium">{d.label}</td>
                      <td className="p-2 text-center">{d.students}</td>
                      <td className="p-2 text-center">{d.classique}</td>
                      <td className="p-2 text-center">{d.sansPorc}</td>
                      <td className="p-2 text-center">{d.vegetarien}</td>
                      <td className="p-2 text-center">
                        {data.canWrite && !data.purged ? (
                          <input
                            type="number"
                            min={0}
                            value={adults[d.key] ?? "0"}
                            onChange={(e) => setAdults((a) => ({ ...a, [d.key]: e.target.value }))}
                            className="w-16 p-1 border rounded text-center"
                            aria-label={`Repas adultes ${d.label}`}
                          />
                        ) : (
                          d.adults
                        )}
                      </td>
                      <td className="p-2 text-center font-bold">{d.total}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="text-[11px] text-slate-500">
              {data.kitchen.sentAt
                ? `Commande envoyée au chef le ${new Date(data.kitchen.sentAt).toLocaleDateString("fr-FR")}.`
                : "Commande pas encore envoyée : ajoutez les repas adultes puis envoyez."}
            </p>
            {data.canWrite && !data.purged && (
              <div className="flex gap-2 flex-wrap">
                <TripButton variant="secondary" size="sm" onClick={() => void sendCuisine(false)} disabled={busy}>
                  Enregistrer les repas adultes
                </TripButton>
                <TripButton size="sm" onClick={() => void sendCuisine(true)} disabled={busy}>
                  {data.kitchen.sentAt ? "Renvoyer la commande au chef (annule et remplace)" : "Envoyer la commande au chef"}
                </TripButton>
              </div>
            )}
          </div>
        )}

        {data.closed && (
          <div className="space-y-2">
            <p className="text-sm font-bold">Liste officielle ({data.official.length})</p>
            <p className="text-xs text-slate-600">
              {data.official.map((o) => `${o.studentName}${o.classe ? ` (${o.classe})` : ""}`).join(" · ") || "—"}
            </p>
            {data.withdrawn.length > 0 && (
              <p className="text-xs text-slate-600">
                Désistés : {data.withdrawn.map((o) => `${o.studentName}${o.classe ? ` (${o.classe})` : ""}`).join(" · ")}
              </p>
            )}
            {data.canWrite && !data.purged && data.official.length > 0 && (
              <div className="flex gap-2 flex-wrap items-center">
                <select
                  value={withdrawIne}
                  onChange={(e) => setWithdrawIne(e.target.value)}
                  className="p-2 border rounded-lg text-sm"
                  aria-label="Élève qui se désiste"
                >
                  <option value="">Désistement après clôture…</option>
                  {data.official.map((o) => (
                    <option key={o.ine} value={o.ine}>
                      {o.studentName}
                    </option>
                  ))}
                </select>
                <input
                  value={withdrawNote}
                  onChange={(e) => setWithdrawNote(e.target.value)}
                  placeholder="Motif (facultatif)"
                  maxLength={300}
                  className="p-2 border rounded-lg text-sm flex-1 min-w-[160px]"
                />
                <TripButton variant="warning" size="sm" onClick={() => void withdraw()} disabled={busy || !withdrawIne}>
                  Noter le désistement
                </TripButton>
              </div>
            )}
          </div>
        )}
      </div>
    </TripSection>
  );
}

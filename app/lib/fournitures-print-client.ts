"use client";

import type { FournituresChild } from "@/app/lib/fournitures-types";

type SuppliesPrintPayload = {
  children: FournituresChild[];
  suppliesByChild: Record<string, Array<{ title: string; items: string[] }>>;
};

/** Ouvre le PDF (même rendu que l'e-mail) dans un nouvel onglet — sans blob: ni iframe (compatible CSP). */
export function openSuppliesPdfForPrint(payload: SuppliesPrintPayload): boolean {
  try {
    const form = document.createElement("form");
    form.method = "POST";
    form.action = "/api/supplies/pdf";
    form.target = "_blank";
    form.acceptCharset = "UTF-8";
    form.style.display = "none";

    const input = document.createElement("input");
    input.type = "hidden";
    input.name = "payload";
    input.value = JSON.stringify(payload);
    form.appendChild(input);

    document.body.appendChild(form);
    form.submit();
    form.remove();
    return true;
  } catch {
    return false;
  }
}

/** Soumet le PDF dans la fenêtre courante (page /simulateurFournitures/print). */
export function submitSuppliesPdfInCurrentWindow(payloadJson: string): void {
  const form = document.createElement("form");
  form.method = "POST";
  form.action = "/api/supplies/pdf";
  form.target = "_self";
  form.acceptCharset = "UTF-8";

  const input = document.createElement("input");
  input.type = "hidden";
  input.name = "payload";
  input.value = payloadJson;
  form.appendChild(input);

  document.body.appendChild(form);
  form.submit();
}

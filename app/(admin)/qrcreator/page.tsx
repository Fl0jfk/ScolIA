"use client";

import { useState, useRef, useEffect } from "react";
import QRCode from "qrcode";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

export default function QRCreator() {
  const [url, setUrl] = useState("https://laprovidence-nicolasbarre.fr/");
  const [fillColor, setFillColor] = useState("#000000");
  const [backColor, setBackColor] = useState("#ffffff");
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const generateQRCodeWithLogo = () => {
    if (canvasRef.current) {
      const canvas = canvasRef.current;
      const ctx = canvas.getContext("2d");
      if (ctx) {
        QRCode.toCanvas(
          canvas,
          url,
          { width: 256, errorCorrectionLevel: "H", color: { dark: fillColor, light: backColor }, margin: 4 },
          (error) => {
            if (error) console.error(error);
          },
        );
      }
    }
  };
  useEffect(() => {
    const fromQuery = new URLSearchParams(window.location.search).get("url");
    if (fromQuery) setUrl(fromQuery);
  }, []);

  useEffect(() => {
    generateQRCodeWithLogo();
  }, [url, fillColor, backColor]);
  const handleUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setUrl(e.target.value);
  };
  const handleDownloadQRCode = () => {
    if (canvasRef.current) {
      const canvas = canvasRef.current;
      const dataUrl = canvas.toDataURL("image/png");
      const link = document.createElement("a");
      link.href = dataUrl;
      link.download = "qr_code_avec_logo.png";
      link.click();
    }
  };
  return (
    <ModulePageShell maxWidthClass="max-w-[1000px]">
      <ModulePageHeader
        eyebrow="Services"
        title="Création de QR Code"
        description="Générez un QR code personnalisé aux couleurs de l'établissement."
      />
      <ModuleCard bodyClassName="flex flex-col gap-5 p-5 sm:p-6">
        <div>
          <label htmlFor="url" className={`mb-2 block ${dash.fieldLabel}`}>
            Adresse URL
          </label>
          <input
            type="text"
            id="url"
            value={url}
            onChange={handleUrlChange}
            className={dash.field}
            placeholder="Entrez l'adresse URL"
          />
        </div>
        <div className="flex flex-wrap items-center gap-4">
          <label htmlFor="fillColor" className={dash.fieldLabel}>
            Couleur du QR Code
          </label>
          <input
            type="color"
            id="fillColor"
            value={fillColor}
            onChange={(e) => setFillColor(e.target.value)}
            className="h-10 w-14 cursor-pointer rounded-xl border border-white/70 bg-white p-1"
            style={{ backgroundColor: fillColor }}
          />
        </div>
        <div className="flex flex-wrap items-center gap-4">
          <label htmlFor="backColor" className={dash.fieldLabel}>
            Couleur de fond
          </label>
          <input
            type="color"
            id="backColor"
            value={backColor}
            onChange={(e) => setBackColor(e.target.value)}
            style={{ backgroundColor: backColor }}
            className="h-10 w-14 cursor-pointer rounded-xl border border-white/70 bg-white p-1"
          />
        </div>
        <canvas ref={canvasRef} width={256} height={256} className="self-center rounded-2xl bg-white/80 p-3 shadow-sm" />
        <ModuleButton onClick={handleDownloadQRCode} className="self-start px-6 py-3">
          Télécharger le QR code
        </ModuleButton>
      </ModuleCard>
    </ModulePageShell>
  );
}

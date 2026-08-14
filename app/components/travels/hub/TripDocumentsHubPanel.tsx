"use client";

import type { RefObject } from "react";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripButton, TripDocumentChip, TripSection } from "@/app/components/travels/TripDetailUI";

export type TripDocumentsHubPanelProps = {
  trip: TravelsTrip;
  isEditing: boolean;
  editedData: { attachments?: Array<{ name: string; url: string; s3Key?: string }> };
  documentCount: number;
  canSign: boolean;
  isOwner: boolean;
  loadingAction: string | null;
  handleRegenerateCircular: () => void;
  canAddDocuments: boolean;
  fileInputRef: RefObject<HTMLInputElement | null>;
  handleFileUpload: (e: React.ChangeEvent<HTMLInputElement>) => void;
  uploading: boolean;
  openSecureFile: (url: string, key?: string | null) => void;
  canSeeTravelDocHoverActions: boolean;
  prepareSendToZeendoc: (file: { name: string; url: string }) => void;
  zeendocSendingUrl: string | null;
  canManageFiles: boolean;
  removeFile: (idx: number) => void;
  withBusLogistics: boolean;
  deleteBusQuote: (quote: { id?: string; fileUrl?: string; providerName?: string; s3KeyIncoming?: string }) => void;
};

export function TripDocumentsHubPanel(p: TripDocumentsHubPanelProps) {
  const {
    trip, isEditing, editedData, documentCount, canSign, isOwner, loadingAction,
    handleRegenerateCircular, canAddDocuments, fileInputRef, handleFileUpload, uploading,
    openSecureFile, canSeeTravelDocHoverActions, prepareSendToZeendoc, zeendocSendingUrl,
    canManageFiles, removeFile, withBusLogistics, deleteBusQuote,
  } = p;
  return (
        <TripSection title="Documents du dossier" subtitle="Pièces jointes, devis transport et circulaire" icon="📁">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-6">
            <p className="text-sm text-slate-600">{documentCount} document{documentCount > 1 ? "s" : ""} au total</p>
            <div className="flex flex-wrap gap-2">
              {trip.status === "VALIDE" && (canSign || isOwner) && (
                <TripButton variant="secondary" size="sm" onClick={handleRegenerateCircular} disabled={!!loadingAction}>
                  {loadingAction === "regenerate-circular" ? "Génération…" : "Régénérer circulaire"}
                </TripButton>
              )}
              {canAddDocuments && (
                <>
                  <input type="file" ref={fileInputRef} className="hidden" onChange={handleFileUpload} />
                  <TripButton variant="primary" size="sm" onClick={() => fileInputRef.current?.click()} disabled={uploading}>
                    {uploading ? "Upload…" : "+ Document"}
                  </TripButton>
                </>
              )}
            </div>
          </div>

          {documentCount === 0 ? (
            <p className="text-sm text-slate-400 italic py-8 text-center">Aucun document dans ce dossier.</p>
          ) : (
            <div className="space-y-8">
              {((isEditing ? editedData.attachments : trip.data.attachments) || []).length > 0 && (
                <div>
                  <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400 mb-3">Pièces jointes</p>
                  <div className="flex flex-wrap gap-2">
                    {((isEditing ? editedData.attachments : trip.data.attachments) || []).map((file: {
                      name: string;
                      url: string;
                      s3Key?: string;
                    }, idx: number) => (
                      <TripDocumentChip
                        key={`att_${idx}`}
                        name={file.name}
                        onOpen={() => openSecureFile(file.url, file.s3Key)}
                        onZeendoc={canSeeTravelDocHoverActions ? () => prepareSendToZeendoc(file) : undefined}
                        zeendocBusy={zeendocSendingUrl === file.url}
                        showZeendoc={canSeeTravelDocHoverActions}
                        onRemove={canManageFiles ? () => removeFile(idx) : undefined}
                        canRemove={canManageFiles}
                      />
                    ))}
                  </div>
                </div>
              )}

              {withBusLogistics && ((trip.receivedDevis?.length || 0) > 0 || trip.data.signedQuoteUrl) && (
                <div>
                  <p className="text-[10px] font-bold uppercase tracking-wider text-amber-700 mb-3">Devis transport bus</p>
                  <div className="flex flex-wrap gap-2">
                    {(trip.receivedDevis || []).map((quote: {
                      id?: string;
                      providerName?: string;
                      fileUrl?: string;
                      s3KeyIncoming?: string;
                    }, idx: number) => {
                      const selected = trip.data.selectedBusQuote?.fileUrl === quote.fileUrl;
                      const label = `🚌 ${quote.providerName || "Transporteur"}${selected ? " (retenu)" : ""}`;
                      return (
                        <TripDocumentChip
                          key={quote.id || `devis_${idx}`}
                          name={label}
                          onOpen={() => quote.fileUrl && openSecureFile(quote.fileUrl, quote.s3KeyIncoming)}
                          onZeendoc={
                            quote.fileUrl && canSeeTravelDocHoverActions
                              ? () => prepareSendToZeendoc({ name: label, url: quote.fileUrl! })
                              : undefined
                          }
                          zeendocBusy={zeendocSendingUrl === quote.fileUrl}
                          showZeendoc={canSeeTravelDocHoverActions}
                          onRemove={canSign && quote.id ? () => deleteBusQuote(quote) : undefined}
                          canRemove={canSign}
                        />
                      );
                    })}
                    {trip.data.signedQuoteUrl && (
                      <TripDocumentChip
                        key="signed_bus"
                        name="🚌 Devis bus signé"
                        onOpen={() => openSecureFile(trip.data.signedQuoteUrl!)}
                        onZeendoc={
                          canSeeTravelDocHoverActions
                            ? () =>
                                prepareSendToZeendoc({
                                  name: "Devis bus signé",
                                  url: trip.data.signedQuoteUrl!,
                                })
                            : undefined
                        }
                        zeendocBusy={zeendocSendingUrl === trip.data.signedQuoteUrl}
                        showZeendoc={canSeeTravelDocHoverActions}
                      />
                    )}
                  </div>
                  {(trip.receivedDevis?.length || 0) > 0 && (
                    <p className="text-[10px] text-slate-500 mt-2">
                      Choix et validation des devis : onglet Transport.
                    </p>
                  )}
                </div>
              )}
            </div>
          )}
        </TripSection>

  );
}

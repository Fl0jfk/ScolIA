import './globals.css';
import { headers } from "next/headers";
import ChatbotBubbleClient from "./components/ChatbotBubbleClient";
import InteractiveCursor from "./components/InteractiveCursor";
import PortalMemoryOnSignOut from "./components/PortalMemoryOnSignOut";
import MessagingOverlayClient from "./components/messaging/MessagingOverlayClient";
import { AppUserProvider } from "./hooks/useAppUser";

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  // nonce réservé CSP (headers middleware)
  void (await headers()).get("x-nonce");
  return (
    <html lang="fr">
      <body>
        <AppUserProvider>
          <InteractiveCursor />
          <PortalMemoryOnSignOut />
          {children}
          <ChatbotBubbleClient />
          <MessagingOverlayClient />
        </AppUserProvider>
      </body>
    </html>
  );
}

import './globals.css';
import { headers } from "next/headers";
import ChatbotBubbleClient from "./components/ChatbotBubbleClient";
import InteractiveCursor from "./components/InteractiveCursor";
import PortalMemoryOnSignOut from "./components/PortalMemoryOnSignOut";
import TenantClerkProvider from "./components/TenantClerkProvider";
import TeamsChatOverlayClient from "./components/teams-chat/TeamsChatOverlayClient";

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const nonce = (await headers()).get("x-nonce") ?? undefined;
  return (
    <html lang="fr">
      <body>
        <TenantClerkProvider nonce={nonce}>
          <InteractiveCursor />
          <PortalMemoryOnSignOut />
          {children}
          <ChatbotBubbleClient/>
          <TeamsChatOverlayClient />
        </TenantClerkProvider>
      </body>
    </html>
  );
}
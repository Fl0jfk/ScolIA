import './globals.css';
import ChatbotBubbleClient from "./components/ChatbotBubbleClient";
import PortalMemoryOnSignOut from "./components/PortalMemoryOnSignOut";
import TenantClerkProvider from "./components/TenantClerkProvider";
import TeamsChatOverlayClient from "./components/teams-chat/TeamsChatOverlayClient";

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="fr">
      <body>
        <TenantClerkProvider>
          <PortalMemoryOnSignOut />
          {children}
          <ChatbotBubbleClient/>
          <TeamsChatOverlayClient />
        </TenantClerkProvider>
      </body>
    </html>
  );
}
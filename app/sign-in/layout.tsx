import SignInHeader from "@/app/components/Header/SignInHeader";
import ScolaAmbientBackground from "@/app/components/ScolaAmbientBackground";
import { AdminBootstrapProvider } from "@/app/contexts/admin-bootstrap";

export default function SignInLayout({ children }: { children: React.ReactNode }) {
  return (
    <ScolaAmbientBackground>
      <AdminBootstrapProvider enableOverlay={false}>
        <SignInHeader />
        {children}
      </AdminBootstrapProvider>
    </ScolaAmbientBackground>
  );
}

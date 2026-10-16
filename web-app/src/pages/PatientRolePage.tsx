import { Navigate } from "react-router-dom";
import { homeFor, ROUTES } from "../auth/RequireRole";
import { useSession } from "../auth/useSession";
import { LogoutButton } from "../components/LogoutButton";
import { vi } from "../i18n/vi";

/** Shown to a patient account: the web app is for clinicians and admins only. */
export function PatientRolePage() {
  const session = useSession();
  if (!session) return <Navigate to={ROUTES.login} replace />;
  if (session.claims.role !== "patient") return <Navigate to={homeFor(session.claims.role)} replace />;
  return (
    <div className="center-page">
      <div className="card login">
        <h1>{vi.patientRole.title}</h1>
        <p>{vi.patientRole.body}</p>
        <LogoutButton />
      </div>
    </div>
  );
}

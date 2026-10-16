import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ApiError } from "./api/http";
import { homeFor, RequireRole, ROUTES } from "./auth/RequireRole";
import { getSession, subscribeSession } from "./auth/session";
import { useSession } from "./auth/useSession";
import { ToastProvider } from "./components/Toasts";
import { AdminLayout, ADMIN_ROUTES } from "./pages/admin/AdminLayout";
import { AuditPage, DevicesPage, FlPage, OverviewPage, ReleasesPage, UsersPage } from "./pages/admin/AdminPages";
import { AlertsPage } from "./pages/clinician/AlertsPage";
import { ClinicianLayout } from "./pages/clinician/ClinicianLayout";
import { PatientsPage } from "./pages/clinician/PatientsPage";
import { LoginPage } from "./pages/LoginPage";
import { PatientRolePage } from "./pages/PatientRolePage";

/**
 * Create the query client: no retry on 4xx, since those will not change.
 *
 * @returns A new query client.
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: (count, error) => !(error instanceof ApiError && error.status < 500) && count < 2,
        refetchOnWindowFocus: true,
      },
    },
  });
}

function Home() {
  const session = useSession();
  return <Navigate to={session ? homeFor(session.claims.role) : ROUTES.login} replace />;
}

/** Route table of the web app. */
export function AppRoutes() {
  return (
    <Routes>
      <Route path={ROUTES.login} element={<LoginPage />} />
      <Route path={ROUTES.patientRole} element={<PatientRolePage />} />
      <Route
        element={
          <RequireRole role="clinician">
            <ClinicianLayout />
          </RequireRole>
        }
      >
        <Route path={ROUTES.patients} element={<PatientsPage />} />
        <Route path={`${ROUTES.patients}/:userId`} element={<PatientsPage />} />
        <Route path={ROUTES.alerts} element={<AlertsPage />} />
      </Route>
      <Route
        path={ROUTES.admin}
        element={
          <RequireRole role="admin">
            <AdminLayout />
          </RequireRole>
        }
      >
        <Route index element={<OverviewPage />} />
        <Route path={ADMIN_ROUTES.devices} element={<DevicesPage />} />
        <Route path={ADMIN_ROUTES.users} element={<UsersPage />} />
        <Route path={ADMIN_ROUTES.releases} element={<ReleasesPage />} />
        <Route path={ADMIN_ROUTES.fl} element={<FlPage />} />
        <Route path={ADMIN_ROUTES.audit} element={<AuditPage />} />
      </Route>
      <Route path="*" element={<Home />} />
    </Routes>
  );
}

/** Root component: query client, toasts and router. */
export function App() {
  const [client] = useState(createQueryClient);
  // Drop cached health data as soon as the session ends (logout or failed refresh).
  useEffect(
    () =>
      subscribeSession(() => {
        if (!getSession()) client.clear();
      }),
    [client],
  );
  return (
    <QueryClientProvider client={client}>
      <ToastProvider>
        <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
          <AppRoutes />
        </BrowserRouter>
      </ToastProvider>
    </QueryClientProvider>
  );
}

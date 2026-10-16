import { Outlet } from "react-router-dom";
import { ROUTES } from "../../auth/RequireRole";
import { AppShell } from "../../components/AppShell";
import { LogoutButton } from "../../components/LogoutButton";
import { vi } from "../../i18n/vi";

/** Paths of the admin pages. */
export const ADMIN_ROUTES = {
  overview: ROUTES.admin,
  devices: `${ROUTES.admin}/thiet-bi`,
  users: `${ROUTES.admin}/nguoi-dung`,
  releases: `${ROUTES.admin}/phat-hanh`,
  fl: `${ROUTES.admin}/federated-learning`,
  audit: `${ROUTES.admin}/audit-log`,
} as const;

/** Frame of the admin pages. */
export function AdminLayout() {
  return (
    <AppShell
      brand={vi.adminNav.brand}
      items={[
        { to: ADMIN_ROUTES.overview, label: vi.adminNav.overview, end: true },
        { to: ADMIN_ROUTES.devices, label: vi.adminNav.devices },
        { to: ADMIN_ROUTES.users, label: vi.adminNav.users },
        { to: ADMIN_ROUTES.releases, label: vi.adminNav.releases },
        { to: ADMIN_ROUTES.fl, label: vi.adminNav.fl },
        { to: ADMIN_ROUTES.audit, label: vi.adminNav.audit },
      ]}
      footer={
        <>
          <span className="muted small">{vi.roles.admin}</span>
          <LogoutButton />
        </>
      }
    >
      <Outlet />
    </AppShell>
  );
}

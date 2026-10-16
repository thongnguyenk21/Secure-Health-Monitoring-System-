import type { ReactNode } from "react";
import { Navigate, useLocation } from "react-router-dom";
import type { Role } from "../api/types";
import { useSession } from "./useSession";

/** Paths of the login page and of each role's home. */
export const ROUTES = {
  login: "/dang-nhap",
  patients: "/benh-nhan",
  alerts: "/canh-bao",
  admin: "/quan-tri",
  patientRole: "/chi-danh-cho-nhan-vien",
} as const;

/**
 * Home path of a role.
 *
 * @param role - Role from the access token.
 * @returns Where that role lands after login.
 */
export function homeFor(role: Role): string {
  if (role === "admin") return ROUTES.admin;
  if (role === "clinician") return ROUTES.patients;
  return ROUTES.patientRole;
}

/** Props of {@link RequireRole}. */
export interface RequireRoleProps {
  /** Role allowed to see the children. */
  role: Role;
  /** Protected content. */
  children: ReactNode;
}

/**
 * Render the children only for a logged-in user of the given role.
 *
 * @param props - Allowed role and content.
 * @returns The content, or a redirect to the login page or the role's home.
 */
export function RequireRole({ role, children }: RequireRoleProps) {
  const session = useSession();
  const location = useLocation();
  if (!session) return <Navigate to={ROUTES.login} replace state={{ from: location.pathname }} />;
  if (session.claims.role !== role) return <Navigate to={homeFor(session.claims.role)} replace />;
  return <>{children}</>;
}

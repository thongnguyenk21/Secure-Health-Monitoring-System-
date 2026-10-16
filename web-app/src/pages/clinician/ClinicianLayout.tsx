import { useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import { Outlet } from "react-router-dom";
import { queryKeys, useMe, useOpenAlerts, usePatients } from "../../api/queries";
import type { LiveEvent } from "../../api/types";
import { ROUTES } from "../../auth/RequireRole";
import { AppShell } from "../../components/AppShell";
import { LogoutButton } from "../../components/LogoutButton";
import { useToast } from "../../components/Toasts";
import { vi } from "../../i18n/vi";
import { useLiveAlerts } from "../../live/useLiveAlerts";
import { alertTypeLabel } from "../../lib/status";
import { shortDeviceId } from "../../lib/format";

/** Frame of the clinician pages: navigation, live alert stream and toasts. */
export function ClinicianLayout() {
  const qc = useQueryClient();
  const toast = useToast();
  const openAlerts = useOpenAlerts();
  const patients = usePatients();
  const me = useMe();

  const onEvent = useCallback(
    (event: LiveEvent) => {
      void qc.invalidateQueries({ queryKey: queryKeys.patients });
      void qc.invalidateQueries({ queryKey: queryKeys.alerts });
      if (event.type === "alert_opened") {
        const patient = patients.data?.find((p) => p.device_id === event.alert.device_id);
        const who = patient?.subject_code ?? shortDeviceId(event.alert.device_id);
        toast(vi.toast.newAlert(who, alertTypeLabel(event.alert.alert_type)));
      }
    },
    [qc, toast, patients.data],
  );
  useLiveAlerts(onEvent);

  const openCount = openAlerts.data?.items.length ?? 0;
  const badge = openCount > 0 ? `${openCount}${openAlerts.data?.next_cursor ? "+" : ""}` : null;

  return (
    <AppShell
      brand={vi.clinicianNav.brand}
      items={[
        { to: ROUTES.patients, label: vi.clinicianNav.patients },
        { to: ROUTES.alerts, label: vi.clinicianNav.alerts, badge },
      ]}
      footer={
        <>
          <span className="nav-user">
            {me.data?.display_name ? <b>{me.data.display_name}</b> : null}
            <span className="muted small">{vi.roles.clinician}</span>
          </span>
          <LogoutButton />
        </>
      }
    >
      <Outlet />
    </AppShell>
  );
}

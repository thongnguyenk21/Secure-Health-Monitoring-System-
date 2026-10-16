import { Link } from "react-router-dom";
import { useAckAlert, useOpenAlerts, usePatients } from "../../api/queries";
import { ROUTES } from "../../auth/RequireRole";
import { Dot, Empty, errorText, Pill, QueryStatus } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { formatDateTime, shortDeviceId } from "../../lib/format";
import { alertTypeLabel, severityColor, severityLabel } from "../../lib/status";
import { formatAlertValue } from "./PatientDetail";

/** Every open alert of the tenant, newest first, with an acknowledge button per row. */
export function AlertsPage() {
  const alerts = useOpenAlerts();
  const patients = usePatients();
  const ack = useAckAlert();
  const byDevice = new Map((patients.data ?? []).filter((p) => p.device_id).map((p) => [p.device_id, p]));
  const items = alerts.data?.items ?? [];

  return (
    <div className="page">
      <h1 className="page-title">{vi.alertsPage.title(items.length)}</h1>
      <QueryStatus isLoading={alerts.isLoading} error={alerts.error} />
      {alerts.data && items.length === 0 ? <Empty>{vi.alertsPage.empty}</Empty> : null}
      {items.length > 0 ? (
        <table className="ptab wide">
          <thead>
            <tr>
              <th>{vi.alertsPage.colPatient}</th>
              <th>{vi.alertsPage.colType}</th>
              <th>{vi.alertsPage.colSeverity}</th>
              <th>{vi.alertsPage.colTime}</th>
              <th>{vi.alertsPage.colValue}</th>
              <th aria-hidden="true" />
            </tr>
          </thead>
          <tbody>
            {items.map((a) => {
              const patient = byDevice.get(a.device_id);
              const busy = ack.isPending && ack.variables === a.id;
              return (
                <tr key={a.id}>
                  <td>
                    {patient ? (
                      <Link to={`${ROUTES.patients}/${patient.user_id}`}>
                        <b>{patient.subject_code}</b>
                      </Link>
                    ) : (
                      <span className="muted">
                        {vi.alertsPage.unassigned}
                        {vi.common.sep}
                        {shortDeviceId(a.device_id)}
                      </span>
                    )}
                    {patient?.display_name ? <div>{patient.display_name}</div> : null}
                  </td>
                  <td>{alertTypeLabel(a.alert_type)}</td>
                  <td>
                    <Dot color={severityColor(a.severity)} />
                    {severityLabel(a.severity)}
                  </td>
                  <td>{formatDateTime(a.raised_at_unix_ms)}</td>
                  <td className="num">{formatAlertValue(a) ?? vi.common.dash}</td>
                  <td className="r">
                    <Pill variant="alert" onClick={() => ack.mutate(a.id)} disabled={busy}>
                      {busy ? vi.detail.acknowledging : vi.detail.acknowledge}
                    </Pill>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : null}
      {ack.error ? <p className="error small">{errorText(ack.error)}</p> : null}
    </div>
  );
}

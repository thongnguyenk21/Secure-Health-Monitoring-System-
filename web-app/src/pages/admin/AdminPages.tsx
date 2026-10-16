import { useState } from "react";
import { useAudit, useDeviceActions, useDevices, useModels, useUsers } from "../../api/queries";
import type { Device, User } from "../../api/types";
import { Dot, Empty, errorText, Pill, QueryStatus } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { formatDate, formatDateTime, shortDeviceId } from "../../lib/format";
import { deviceStatusView } from "../../lib/status";
import { useNow } from "../../lib/useNow";
import { AddDeviceForm, InviteUserForm } from "./forms";
import {
  actorLabel,
  AuditTile,
  DevicesTile,
  FlTile,
  ReleasesTile,
  SystemTile,
  targetLabel,
  UsersTile,
  usersById,
} from "./tiles";

const AUDIT_PAGE_SIZE = 50;

/** Admin dashboard: the two-column overview of the wireframe. */
export function OverviewPage() {
  return (
    <div className="admin-grid">
      <div className="bcol stack">
        <DevicesTile />
        <ReleasesTile />
        <UsersTile />
      </div>
      <div className="bcol stack">
        <FlTile />
        <AuditTile />
        <SystemTile />
      </div>
    </div>
  );
}

function LabelCell({ device }: { device: Device }) {
  const { label } = useDeviceActions();
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(device.label ?? "");
  if (!editing) {
    return (
      <span className="label-cell">
        {device.label || vi.common.dash}
        {device.status !== "revoked" ? (
          <button type="button" className="link" onClick={() => setEditing(true)}>
            {vi.common.edit}
          </button>
        ) : null}
      </span>
    );
  }
  return (
    <form
      className="label-cell"
      onSubmit={(e) => {
        e.preventDefault();
        label.mutate({ deviceId: device.device_id, label: value.trim() }, { onSuccess: () => setEditing(false) });
      }}
    >
      <input value={value} maxLength={64} onChange={(e) => setValue(e.target.value)} aria-label={vi.devices.colLabel} />
      <button type="submit" className="link" disabled={label.isPending}>
        {vi.common.save}
      </button>
      <button type="button" className="link" onClick={() => setEditing(false)}>
        {vi.common.cancel}
      </button>
    </form>
  );
}

function DeviceRow({ device, patients, now }: { device: Device; patients: readonly User[]; now: number }) {
  const { assign, revoke } = useDeviceActions();
  const view = deviceStatusView(device, now);
  const error = assign.error ?? revoke.error;
  return (
    <tr>
      <td className="mono" title={device.device_id}>
        {shortDeviceId(device.device_id)}
      </td>
      <td>{device.kind === "gateway" ? vi.devices.kindGateway : vi.devices.kindWearable}</td>
      <td>
        <LabelCell device={device} />
      </td>
      <td>
        {device.kind === "wearable" && device.status !== "revoked" ? (
          <select
            value={device.assigned_user_id ?? ""}
            aria-label={vi.devices.assignLabel}
            disabled={assign.isPending}
            onChange={(e) => assign.mutate({ deviceId: device.device_id, userId: e.target.value || null })}
          >
            <option value="">{vi.devices.unassigned}</option>
            {patients.map((p) => (
              <option key={p.id} value={p.id}>
                {p.subject_code ?? p.email}
              </option>
            ))}
          </select>
        ) : (
          (patients.find((p) => p.id === device.assigned_user_id)?.subject_code ?? vi.common.dash)
        )}
        {error ? <div className="error">{errorText(error)}</div> : null}
      </td>
      <td>{device.fw_version ?? vi.common.dash}</td>
      <td>{vi.common.dash}</td>
      <td>
        <Dot color={view.color} />
        {view.label}
      </td>
      <td>{device.last_seen_unix_ms ? formatDateTime(device.last_seen_unix_ms) : vi.common.dash}</td>
      <td className="r">
        {device.status !== "revoked" ? (
          <Pill
            variant="alert"
            disabled={revoke.isPending}
            onClick={() => {
              if (window.confirm(vi.devices.revokeConfirm(shortDeviceId(device.device_id)))) {
                revoke.mutate(device.device_id);
              }
            }}
          >
            {vi.devices.revoke}
          </Pill>
        ) : null}
      </td>
    </tr>
  );
}

/** Full device table with label edit, assignment and revoke. */
export function DevicesPage() {
  const devices = useDevices();
  const patients = useUsers("patient");
  const now = useNow();
  const [adding, setAdding] = useState(false);
  const items = devices.data ?? [];
  return (
    <div className="page">
      <div className="row-between">
        <h1 className="page-title">{vi.devices.title(items.length)}</h1>
        <Pill onClick={() => setAdding((v) => !v)}>{vi.devices.add}</Pill>
      </div>
      {adding ? (
        <div className="tile">
          <AddDeviceForm onDone={() => setAdding(false)} />
        </div>
      ) : null}
      <QueryStatus isLoading={devices.isLoading} error={devices.error} />
      {devices.data && items.length === 0 ? <Empty>{vi.devices.empty}</Empty> : null}
      {items.length > 0 ? (
        <table className="ptab wide">
          <thead>
            <tr>
              <th>{vi.devices.colId}</th>
              <th>{vi.devices.colKind}</th>
              <th>{vi.devices.colLabel}</th>
              <th>{vi.devices.colPatient}</th>
              <th>{vi.devices.colFw}</th>
              <th>{vi.devices.colBattery}</th>
              <th>{vi.devices.colStatus}</th>
              <th>{vi.devices.colLastSeen}</th>
              <th aria-hidden="true" />
            </tr>
          </thead>
          <tbody>
            {items.map((d) => (
              <DeviceRow key={d.device_id} device={d} patients={patients.data ?? []} now={now} />
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  );
}

/** Full user table with the invite form. */
export function UsersPage() {
  const users = useUsers();
  const [inviting, setInviting] = useState(false);
  const [created, setCreated] = useState<string | null>(null);
  const items = users.data ?? [];
  return (
    <div className="page">
      <div className="row-between">
        <h1 className="page-title">{vi.users.pageTitle(items.length)}</h1>
        <Pill onClick={() => setInviting((v) => !v)}>{vi.users.invite}</Pill>
      </div>
      {inviting ? (
        <div className="tile">
          <InviteUserForm
            onDone={(email) => {
              setInviting(false);
              setCreated(email);
            }}
          />
        </div>
      ) : null}
      {created ? <p className="ok small">{vi.users.created(created)}</p> : null}
      <QueryStatus isLoading={users.isLoading} error={users.error} />
      {users.data && items.length === 0 ? <Empty>{vi.users.empty}</Empty> : null}
      {items.length > 0 ? (
        <table className="ptab wide">
          <thead>
            <tr>
              <th>{vi.users.colEmail}</th>
              <th>{vi.users.colName}</th>
              <th>{vi.users.colRole}</th>
              <th>{vi.users.colCode}</th>
              <th>{vi.users.colCreated}</th>
            </tr>
          </thead>
          <tbody>
            {items.map((u) => (
              <tr key={u.id}>
                <td>{u.email}</td>
                <td>{u.display_name ?? vi.common.dash}</td>
                <td>{vi.roles[u.role]}</td>
                <td>{u.subject_code ?? vi.common.dash}</td>
                <td>{formatDate(u.created_at_unix_ms)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  );
}

/** Table of signed model versions. */
export function ReleasesPage() {
  const models = useModels();
  const items = models.data ?? [];
  return (
    <div className="page">
      <h1 className="page-title">{vi.releases.title}</h1>
      <QueryStatus isLoading={models.isLoading} error={models.error} />
      {models.data && items.length === 0 ? <Empty>{vi.releases.empty}</Empty> : null}
      {items.length > 0 ? (
        <table className="ptab wide">
          <thead>
            <tr>
              <th>{vi.releases.colVersion}</th>
              <th>{vi.releases.colSha}</th>
              <th>{vi.releases.colMinFw}</th>
              <th>{vi.releases.colCreated}</th>
              <th>{vi.releases.colTarget}</th>
              <th>{vi.releases.colNotes}</th>
            </tr>
          </thead>
          <tbody>
            {items.map((m) => (
              <tr key={m.version}>
                <td>
                  <b>{m.version}</b>
                </td>
                <td className="mono" title={m.sha256}>
                  {`${m.sha256.slice(0, 16)}${vi.common.ellipsis}`}
                </td>
                <td>{m.min_fw ?? vi.common.dash}</td>
                <td>{formatDate(m.created_at_unix_ms)}</td>
                <td>{m.is_target ? <Pill variant="accent">{vi.releases.target}</Pill> : vi.common.dash}</td>
                <td>{m.notes ?? vi.common.dash}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  );
}

/** Federated Learning page: empty until the FL API exists. */
export function FlPage() {
  return (
    <div className="page">
      <h1 className="page-title">{vi.fl.title}</h1>
      <Empty>{vi.fl.empty}</Empty>
    </div>
  );
}

/** Audit log with cursor pagination (newest first). */
export function AuditPage() {
  // Cursors of the pages visited so far; the last one is the current page.
  const [cursors, setCursors] = useState<(string | null)[]>([null]);
  const cursor = cursors[cursors.length - 1] ?? null;
  const audit = useAudit(AUDIT_PAGE_SIZE, cursor);
  const users = useUsers();
  const byId = usersById(users.data);
  const items = audit.data?.items ?? [];
  const next = audit.data?.next_cursor ?? null;
  return (
    <div className="page">
      <h1 className="page-title">{vi.audit.title}</h1>
      <QueryStatus isLoading={audit.isLoading} error={audit.error} />
      {audit.data && items.length === 0 ? <Empty>{vi.audit.empty}</Empty> : null}
      {items.length > 0 ? (
        <table className="ptab wide">
          <thead>
            <tr>
              <th>{vi.audit.colTime}</th>
              <th>{vi.audit.colActor}</th>
              <th>{vi.audit.colAction}</th>
              <th>{vi.audit.colTarget}</th>
              <th>{vi.audit.colOutcome}</th>
            </tr>
          </thead>
          <tbody>
            {items.map((e) => (
              <tr key={e.id}>
                <td>{formatDateTime(e.at_unix_ms)}</td>
                <td>{actorLabel(e, byId)}</td>
                <td className="mono">{e.action}</td>
                <td title={e.target}>{targetLabel(e, byId) || vi.common.dash}</td>
                <td className={e.outcome === "denied" ? "text-alert" : "text-ok"}>
                  {e.outcome === "denied" ? vi.audit.denied : vi.audit.allowed}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      <div className="pager">
        <button
          type="button"
          className="btn-secondary"
          disabled={cursors.length <= 1}
          onClick={() => setCursors((c) => c.slice(0, -1))}
        >
          {vi.common.previousPage}
        </button>
        <span className="muted small">{vi.common.pageOf(cursors.length)}</span>
        <button
          type="button"
          className="btn-secondary"
          disabled={!next}
          onClick={() => setCursors((c) => [...c, next])}
        >
          {vi.common.nextPage}
        </button>
      </div>
    </div>
  );
}

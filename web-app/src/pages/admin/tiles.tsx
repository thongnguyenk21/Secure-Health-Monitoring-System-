import { useState } from "react";
import { Link } from "react-router-dom";
import { useAudit, useDevices, useModels, useSystemStatus, useUsers } from "../../api/queries";
import type { AuditEntry, Role, User } from "../../api/types";
import { Dot, Empty, Pill, QueryStatus } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { formatDate, formatTime, shortDeviceId } from "../../lib/format";
import { useNow } from "../../lib/useNow";
import { componentColor, deviceStatusView } from "../../lib/status";
import { ADMIN_ROUTES } from "./AdminLayout";
import { AddDeviceForm, InviteUserForm } from "./forms";

// The overview shows the first rows only; the Thiết bị page lists all of them.
const OVERVIEW_DEVICE_ROWS = 6;
const OVERVIEW_AUDIT_ROWS = 10;

/**
 * Index users by id.
 *
 * @param users - Users, or nothing while loading.
 * @returns Map from user id to user.
 */
export function usersById(users: readonly User[] | undefined): Map<string, User> {
  return new Map((users ?? []).map((u) => [u.id, u]));
}

/** Tile "Thiết bị (N)" with the add form and the first devices. */
export function DevicesTile() {
  const devices = useDevices();
  const patients = useUsers("patient");
  const now = useNow();
  const [adding, setAdding] = useState(false);
  const byId = usersById(patients.data);
  const items = devices.data ?? [];
  const rows = items.slice(0, OVERVIEW_DEVICE_ROWS);

  return (
    <section className="tile">
      <div className="row-between">
        <b>{vi.devices.title(items.length)}</b>
        <Pill onClick={() => setAdding((v) => !v)}>{vi.devices.add}</Pill>
      </div>
      {adding ? <AddDeviceForm onDone={() => setAdding(false)} /> : null}
      <QueryStatus isLoading={devices.isLoading} error={devices.error} />
      {devices.data && items.length === 0 ? <Empty>{vi.devices.empty}</Empty> : null}
      {rows.length > 0 ? (
        <table className="ptab">
          <thead>
            <tr>
              <th>{vi.devices.colId}</th>
              <th>{vi.devices.colPatient}</th>
              <th>{vi.devices.colFw}</th>
              <th>{vi.devices.colBattery}</th>
              <th>{vi.devices.colStatus}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((d) => {
              const view = deviceStatusView(d, now);
              const patient = d.assigned_user_id ? byId.get(d.assigned_user_id) : undefined;
              return (
                <tr key={d.device_id}>
                  <td className="mono" title={d.device_id}>
                    {shortDeviceId(d.device_id)}
                  </td>
                  <td>{d.assigned_user_id ? (patient?.subject_code ?? shortDeviceId(d.assigned_user_id)) : vi.devices.unassigned}</td>
                  <td>{d.fw_version ?? vi.common.dash}</td>
                  <td>{vi.common.dash}</td>
                  <td>
                    <Dot color={view.color} />
                    {view.label}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : null}
      {items.length > rows.length ? (
        <Link className="more-link" to={ADMIN_ROUTES.devices}>
          {vi.devices.more(items.length - rows.length)}
        </Link>
      ) : null}
    </section>
  );
}

/** Tile "Phát hành OTA" listing the model versions. */
export function ReleasesTile() {
  const models = useModels();
  return (
    <section className="tile">
      <b>{vi.releases.title}</b>
      <QueryStatus isLoading={models.isLoading} error={models.error} />
      {models.data && models.data.length === 0 ? <Empty>{vi.releases.empty}</Empty> : null}
      {models.data?.slice(0, 4).map((m) => (
        <div key={m.version} className="rel">
          <div>
            <b>{vi.releases.model(m.version)}</b>
            <div>
              {vi.releases.signedLine(`${m.sha256.slice(0, 8)}${vi.common.ellipsis}`, formatDate(m.created_at_unix_ms))}
              {m.min_fw ? vi.releases.minFw(m.min_fw) : null}
            </div>
          </div>
          {m.is_target ? <Pill variant="accent">{vi.releases.target}</Pill> : <span />}
        </div>
      ))}
    </section>
  );
}

const ROLE_ROWS: { role: Role; label: string }[] = [
  { role: "patient", label: vi.users.rolePatient },
  { role: "clinician", label: vi.users.roleClinician },
  { role: "admin", label: vi.users.roleAdmin },
];

/** Tile "Người dùng và vai trò" with counts per role and the invite form. */
export function UsersTile() {
  const users = useUsers();
  const [inviting, setInviting] = useState(false);
  const [created, setCreated] = useState<string | null>(null);
  const count = (role: Role) => (users.data ?? []).filter((u) => u.role === role).length;
  return (
    <section className="tile">
      <div className="row-between">
        <b>{vi.users.title}</b>
        <Pill onClick={() => setInviting((v) => !v)}>{vi.users.invite}</Pill>
      </div>
      {inviting ? (
        <InviteUserForm
          onDone={(email) => {
            setInviting(false);
            setCreated(email);
          }}
        />
      ) : null}
      {created ? <p className="ok small">{vi.users.created(created)}</p> : null}
      <QueryStatus isLoading={users.isLoading} error={users.error} />
      {users.data
        ? ROLE_ROWS.map((r) => (
            <div key={r.role} className="alog">
              <span className="num">{count(r.role)}</span>
              <span>{r.label}</span>
              <em />
            </div>
          ))
        : null}
    </section>
  );
}

/** Tile "Federated Learning": empty until the FL API exists. */
export function FlTile() {
  return (
    <section className="tile">
      <b>{vi.fl.title}</b>
      <Empty>{vi.fl.empty}</Empty>
    </section>
  );
}

/**
 * Human-readable actor of an audit entry.
 *
 * @param entry - Audit entry.
 * @param users - Users by id.
 * @returns Display name, email, short id, or "hệ thống".
 */
export function actorLabel(entry: AuditEntry, users: Map<string, User>): string {
  if (!entry.actor_id) return vi.audit.system;
  const user = users.get(entry.actor_id);
  return user?.display_name ?? user?.email ?? `${entry.actor_id.slice(0, 8)}${vi.common.ellipsis}`;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const DEVICE_ID = /^[0-9a-f]{32}$/;

/**
 * Short, readable target of an audit entry.
 *
 * @param entry - Audit entry.
 * @param users - Users by id.
 * @returns The subject code or name for a user id, a short device id, or the raw target.
 */
export function targetLabel(entry: AuditEntry, users: Map<string, User>): string {
  const target = entry.target ?? "";
  if (UUID.test(target)) {
    const user = users.get(target);
    return user?.subject_code ?? user?.display_name ?? `${target.slice(0, 8)}${vi.common.ellipsis}`;
  }
  if (DEVICE_ID.test(target)) return shortDeviceId(target);
  return target;
}

/** Tile "Audit log" with the latest entries. */
export function AuditTile() {
  const audit = useAudit(OVERVIEW_AUDIT_ROWS, null);
  const users = useUsers();
  const byId = usersById(users.data);
  const items = audit.data?.items ?? [];
  return (
    <section className="tile">
      <b>{vi.audit.title}</b>
      <QueryStatus isLoading={audit.isLoading} error={audit.error} />
      {audit.data && items.length === 0 ? <Empty>{vi.audit.empty}</Empty> : null}
      {items.map((e) => (
        <div key={e.id} className="alog">
          <span>{formatTime(e.at_unix_ms)}</span>
          <span>
            {actorLabel(e, byId)}
            {vi.common.sep}
            {e.action}
            {e.target ? `${vi.common.sep}${targetLabel(e, byId)}` : null}
          </span>
          <em className={e.outcome === "denied" ? "no" : undefined}>
            {e.outcome === "denied" ? vi.audit.denied : vi.audit.allowed}
          </em>
        </div>
      ))}
    </section>
  );
}

/** Tile "Trạng thái thành phần" with one dot per component. */
export function SystemTile() {
  const status = useSystemStatus();
  return (
    <section className="tile">
      <b>{vi.system.title}</b>
      <QueryStatus isLoading={status.isLoading} error={status.error} />
      {status.data && status.data.length === 0 ? <Empty>{vi.system.empty}</Empty> : null}
      <div className="components">
        {status.data?.map((c) => (
          <span key={c.name} title={c.detail}>
            <Dot color={componentColor(c.state)} />
            {c.name}
            {c.detail && c.state !== "ok" ? `: ${c.detail}` : null}
          </span>
        ))}
      </div>
    </section>
  );
}

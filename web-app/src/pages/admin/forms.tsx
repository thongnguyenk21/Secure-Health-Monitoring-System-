import { useState, type FormEvent } from "react";
import { useCreateDevice, useRegisterUser } from "../../api/queries";
import type { Device, Role } from "../../api/types";
import { errorText } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { isDeviceId } from "../../lib/format";

const SUBJECT_CODE = /^[A-Z]{2}-[0-9]{3,6}$/;
const MIN_PASSWORD = 12;

/**
 * Inline form that registers a wearable or gateway.
 *
 * @param props - Called after the device is created.
 */
export function AddDeviceForm({ onDone }: { onDone: () => void }) {
  const create = useCreateDevice();
  const [deviceId, setDeviceId] = useState("");
  const [kind, setKind] = useState<Device["kind"]>("wearable");
  const [label, setLabel] = useState("");
  const [invalid, setInvalid] = useState(false);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    const id = deviceId.trim().toLowerCase();
    if (!isDeviceId(id)) {
      setInvalid(true);
      return;
    }
    setInvalid(false);
    create.mutate({ device_id: id, kind, label: label.trim() || undefined }, { onSuccess: onDone });
  };

  return (
    <form className="inline-form" onSubmit={onSubmit}>
      <label>
        {vi.devices.formDeviceId}
        <input value={deviceId} maxLength={32} onChange={(e) => setDeviceId(e.target.value)} required />
      </label>
      <label>
        {vi.devices.formKind}
        <select value={kind} onChange={(e) => setKind(e.target.value as Device["kind"])}>
          <option value="wearable">{vi.devices.kindWearable}</option>
          <option value="gateway">{vi.devices.kindGateway}</option>
        </select>
      </label>
      <label>
        {vi.devices.formLabel}
        <input value={label} maxLength={64} onChange={(e) => setLabel(e.target.value)} />
      </label>
      {invalid ? <p className="error small">{vi.devices.formInvalidId}</p> : null}
      {create.error ? <p className="error small">{errorText(create.error)}</p> : null}
      <div className="form-actions">
        <button type="button" className="btn-secondary" onClick={onDone}>
          {vi.common.cancel}
        </button>
        <button type="submit" className="btn-primary" disabled={create.isPending}>
          {vi.devices.formSubmit}
        </button>
      </div>
    </form>
  );
}

/**
 * Inline form that creates a user account through `/auth/register`.
 *
 * @param props - Called after the account is created, with its email.
 */
export function InviteUserForm({ onDone }: { onDone: (email: string | null) => void }) {
  const register = useRegisterUser();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("clinician");
  const [displayName, setDisplayName] = useState("");
  const [subjectCode, setSubjectCode] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (password.length < MIN_PASSWORD) {
      setProblem(vi.users.formPasswordShort);
      return;
    }
    const code = subjectCode.trim().toUpperCase();
    if (role === "patient" && code && !SUBJECT_CODE.test(code)) {
      setProblem(vi.users.formCodeInvalid);
      return;
    }
    setProblem(null);
    register.mutate(
      {
        email: email.trim(),
        password,
        role,
        display_name: displayName.trim() || undefined,
        subject_code: role === "patient" && code ? code : undefined,
      },
      { onSuccess: (user) => onDone(user.email) },
    );
  };

  return (
    <form className="inline-form" onSubmit={onSubmit}>
      <label>
        {vi.users.formEmail}
        <input type="email" value={email} maxLength={256} onChange={(e) => setEmail(e.target.value)} required />
      </label>
      <label>
        {vi.users.formPassword}
        <input
          type="password"
          autoComplete="new-password"
          value={password}
          maxLength={256}
          onChange={(e) => setPassword(e.target.value)}
          required
        />
      </label>
      <label>
        {vi.users.formRole}
        <select value={role} onChange={(e) => setRole(e.target.value as Role)}>
          <option value="clinician">{vi.roles.clinician}</option>
          <option value="patient">{vi.roles.patient}</option>
          <option value="admin">{vi.roles.admin}</option>
        </select>
      </label>
      <label>
        {vi.users.formName}
        <input value={displayName} maxLength={64} onChange={(e) => setDisplayName(e.target.value)} />
      </label>
      {role === "patient" ? (
        <label>
          {vi.users.formCode}
          <input value={subjectCode} maxLength={16} onChange={(e) => setSubjectCode(e.target.value)} />
        </label>
      ) : null}
      {problem ? <p className="error small">{problem}</p> : null}
      {register.error ? <p className="error small">{errorText(register.error)}</p> : null}
      <div className="form-actions">
        <button type="button" className="btn-secondary" onClick={() => onDone(null)}>
          {vi.common.cancel}
        </button>
        <button type="submit" className="btn-primary" disabled={register.isPending}>
          {vi.users.formSubmit}
        </button>
      </div>
    </form>
  );
}

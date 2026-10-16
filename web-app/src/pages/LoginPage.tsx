import { useState, type FormEvent } from "react";
import { Navigate } from "react-router-dom";
import { api } from "../api/client";
import { ApiError } from "../api/http";
import { homeFor } from "../auth/RequireRole";
import { useSession } from "../auth/useSession";
import { vi } from "../i18n/vi";

/**
 * Vietnamese text for a failed login.
 *
 * @param error - Error thrown by the login call.
 * @returns The message under the form.
 */
export function loginErrorText(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 401) return vi.login.wrongCredentials;
    if (error.status === 429) return vi.login.tooManyAttempts;
    return error.message;
  }
  return vi.login.networkError;
}

/** Centered login card; sends clinicians to the patient list and admins to the dashboard. */
export function LoginPage() {
  const session = useSession();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (session) return <Navigate to={homeFor(session.claims.role)} replace />;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.login(email.trim(), password);
    } catch (err) {
      setError(loginErrorText(err));
      setBusy(false);
    }
  };

  return (
    <div className="center-page">
      <form className="card login" onSubmit={(e) => void onSubmit(e)}>
        <h1>{vi.login.title}</h1>
        <p className="muted">{vi.login.subtitle}</p>
        <label>
          {vi.login.email}
          <input type="email" autoComplete="username" required value={email} onChange={(e) => setEmail(e.target.value)} />
        </label>
        <label>
          {vi.login.password}
          <input
            type="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {error ? (
          <p className="error" role="alert">
            {error}
          </p>
        ) : null}
        <button type="submit" className="btn-primary" disabled={busy}>
          {busy ? vi.login.submitting : vi.login.submit}
        </button>
      </form>
    </div>
  );
}

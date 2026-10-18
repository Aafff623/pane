export type CredentialFailure = "network" | "rateLimit" | "credential" | "permission" | "unsupported" | "unknown";

// Unknown responses stay unknown. A failed quota read alone never proves
// that an inference key is invalid or that the subscription is exhausted.
export function credentialFailure(message: string, status = "error"): CredentialFailure {
  if (/\b429\b|rate.?limit|too many requests|限流/i.test(message)) return "rateLimit";
  if (/network|timeout|timed out|dns|failed to connect|connection (?:reset|refused|closed|error)|error sending request|transport|proxy|tls|网络|超时|连接失败|HTTP 5\d\d/i.test(message)) return "network";
  if (/no quota|not published|not supported|no .*headers|unavailable.*api|discontinued.*api|不支持|未公开/i.test(message)) return "unsupported";
  if (status === "needs_setup" || /\b403\b|permission|scope|admin key|management key|billing read|forbidden|权限/i.test(message)) return "permission";
  if (/\b401\b|unauthori[sz]ed|invalid.*key|key.*invalid|rejected|expired|sign.in|log.in|invalid.*(?:cookie|token|credential)|(?:cookie|token|credential).*(?:invalid|missing|expired)|凭据|密钥|登录|过期/i.test(message)) return "credential";
  return "unknown";
}

export function accountFormReady(key: string, label: string, baseUrl: string, needsUrl: boolean): boolean {
  return Boolean(key.trim() && label.trim() && (!needsUrl || baseUrl.trim()));
}

export function enabledAccountCount(entries: readonly { id?: string }[], disabled: readonly string[]): number {
  return entries.filter((entry) => !entry.id || !disabled.includes(entry.id)).length;
}

export type CredentialFailure = "network" | "rateLimit" | "credential" | "permission" | "unsupported" | "subscription" | "empty" | "unknown";

export type CredentialFinality = "revoked" | "transient";

/// Only an explicit revocation makes a credential dead. A generic 401/403,
/// a timeout or a proxy failure is recoverable and must not send the user
/// through a re-login flow (cockpit lesson: transient failures were being
/// promoted to dead accounts).
export function credentialFinality(
  message: string,
  authMode: "oauth" | "api_key" | "local" | "unknown" = "unknown",
): CredentialFinality {
  if (authMode === "api_key" || authMode === "local") return "transient";
  if (/network|timeout|timed out|dns|failed to connect|connection (?:reset|refused|closed|error)|error sending request|transport|proxy|tls|HTTP 5\d\d|429|rate.?limit|网络|超时|连接失败/i.test(message)) {
    return "transient";
  }
  return /token_revoked|token_invalidated|invalid_grant|refresh[_ -]?token.*(?:revoked|invalidated|invalid)|(?:revoked|invalidated).*(?:token|grant)/i.test(message)
    ? "revoked"
    : "transient";
}

// Unknown responses stay unknown. A failed quota read alone never proves
// that an inference key is invalid or that the subscription is exhausted.
// Subscription states are checked before credentials so "subscription
// expired" is not flattened into "sign in again".
export function credentialFailure(message: string, status = "error"): CredentialFailure {
  if (/\b429\b|rate.?limit|too many requests|限流/i.test(message)) return "rateLimit";
  if (/network|timeout|timed out|dns|failed to connect|connection (?:reset|refused|closed|error)|error sending request|transport|proxy|tls|网络|超时|连接失败|HTTP 5\d\d/i.test(message)) return "network";
  if (/(?:subscription|plan|membership).*(?:expired|ended|cancel(?:l)?ed|inactive|not active)|no active (?:subscription|plan)|订阅.*(?:到期|过期|失效|已结束|未生效)|套餐.*(?:到期|过期|失效|已结束)|会员.*(?:到期|过期|失效)|续费/i.test(message)) return "subscription";
  // The request succeeded but produced no displayable windows (StepFun's
  // "no quota data"): that is an empty result, not a rejected credential and
  // not an unsupported API.
  if (/no (?:quota|usage)(?: data| windows)?|returned no (?:quota|usage|windows)|no recognizable|empty (?:quota|usage)|未返回.*额度|没有.*额度数据|无可显示.*额度|未获取到.*额度/i.test(message)) return "empty";
  if (/no quota|not published|not supported|no .*headers|unavailable.*api|(?:api.*discontinued|discontinued.*api)|不支持|未公开/i.test(message)) return "unsupported";
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

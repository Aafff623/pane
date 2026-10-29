export const SITE_REF = "designmode.app";

const SKIP_HOSTS = new Set([
  "designmode.app",
  "schema.org",
  "googletagmanager.com",
  "google-analytics.com",
  "api.producthunt.com",
  "mcp.designmode.app",
  "opensource.org",
]);

function hostnameOf(host: string): string {
  return host.replace(/^www\./i, "").toLowerCase();
}

/** Add ref=designmode.app to external navigational http(s) URLs. */
export function withNavRef(href: string): string {
  if (!/^https?:\/\//i.test(href)) return href;

  let url: URL;
  try {
    url = new URL(href);
  } catch {
    return href;
  }

  if (url.protocol !== "http:" && url.protocol !== "https:") return href;
  if (SKIP_HOSTS.has(hostnameOf(url.hostname))) return href;
  if (url.searchParams.has("ref")) return href;

  const hash = url.hash;
  const base = hash ? href.slice(0, -hash.length) : href;
  const sep = url.search ? "&" : "?";
  return `${base}${sep}ref=${SITE_REF}${hash}`;
}

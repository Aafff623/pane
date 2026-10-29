import { readFile } from "node:fs/promises";

const host = process.env.INDEXNOW_HOST ?? "designmode.app";
const site = `https://${host}`;
const key = (await readFile(new URL("../public/indexnow-key.txt", import.meta.url), "utf8")).trim();

if (!/^[A-Za-z0-9-]{8,128}$/.test(key)) {
  throw new Error("public/indexnow-key.txt is not a valid IndexNow key");
}

const sitemapResponse = await fetch(`${site}/sitemap.xml`, {
  headers: { "user-agent": "Design-Mode-IndexNow/1.0" },
});

if (!sitemapResponse.ok) {
  throw new Error(`Could not fetch production sitemap: HTTP ${sitemapResponse.status}`);
}

const sitemap = await sitemapResponse.text();
const urls = [...sitemap.matchAll(/<loc>(.*?)<\/loc>/g)].map((match) =>
  match[1].replaceAll("&amp;", "&"),
);

if (urls.length === 0) {
  throw new Error("Production sitemap contained no <loc> entries");
}

for (const url of urls) {
  const parsed = new URL(url);
  if (parsed.protocol !== "https:" || parsed.host !== host) {
    throw new Error(`Refusing to submit a URL outside https://${host}: ${url}`);
  }
}

const keyLocation = `${site}/indexnow-key.txt`;
const keyResponse = await fetch(keyLocation, { redirect: "error" });
const deployedKey = keyResponse.ok ? (await keyResponse.text()).trim() : "";

if (deployedKey !== key) {
  throw new Error(
    `Production key verification failed at ${keyLocation}; deploy the website before submitting`,
  );
}

const response = await fetch("https://api.indexnow.org/indexnow", {
  method: "POST",
  headers: { "content-type": "application/json; charset=utf-8" },
  body: JSON.stringify({ host, key, keyLocation, urlList: urls }),
});

if (!response.ok) {
  const detail = (await response.text()).slice(0, 500);
  throw new Error(`IndexNow rejected the submission: HTTP ${response.status} ${detail}`);
}

console.log(`IndexNow accepted ${urls.length} URLs for ${host} (HTTP ${response.status}).`);

// Release gate for the skin unbundle.
//
//   node site/scripts/check-skin-packs.mjs          # local index vs local zips
//   node site/scripts/check-skin-packs.mjs --live   # ... and vs the deployed index
//
// The unbundled app ships without built-in signal/ink, so the deployed zips
// must match the built index before a release. This script makes that state
// checkable instead of a memory exercise.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dir = join(here, "..", "public", "skins");
const LIVE_INDEX = "https://pane.threetwoa.live/skins/index.json";
const wantLive = process.argv.includes("--live");

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const index = JSON.parse(await readFile(join(dir, "index.json"), "utf8"));

let failed = false;
const local = new Map();
for (const pack of index.packs ?? []) {
  const file = pack.url.split("/").pop();
  const bytes = await readFile(join(dir, file));
  const sizeOk = bytes.length === pack.size;
  const shaOk = sha256(bytes) === pack.sha256;
  local.set(pack.id, pack);
  if (!sizeOk || !shaOk) failed = true;
  console.log(
    `${pack.id.padEnd(8)} v${pack.version}  ${sizeOk ? "size ok" : `size ${bytes.length} != ${pack.size}`}  ${shaOk ? "sha ok" : "sha MISMATCH"}  ${file}`,
  );
}
if (!local.size) {
  console.error("index.json lists no packs");
  failed = true;
}

if (wantLive) {
  console.log(`\nlive index: ${LIVE_INDEX}`);
  try {
    const res = await fetch(LIVE_INDEX, { cache: "no-store" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const deployed = await res.json();
    const byId = new Map((deployed.packs ?? []).map((p) => [p.id, p]));
    for (const [id, pack] of local) {
      const remote = byId.get(id);
      if (!remote) {
        console.log(`${id.padEnd(8)} NOT DEPLOYED`);
        failed = true;
      } else if (remote.size !== pack.size || remote.sha256 !== pack.sha256) {
        console.log(`${id.padEnd(8)} DEPLOYED COPY IS STALE (${remote.size} bytes, ${remote.sha256.slice(0, 12)}…)`);
        failed = true;
      } else {
        console.log(`${id.padEnd(8)} deployed copy matches`);
      }
    }
    for (const id of byId.keys()) {
      if (!local.has(id)) console.log(`${id.padEnd(8)} deployed but not in the local index`);
    }
  } catch (error) {
    console.error(`live check failed: ${String(error)}`);
    failed = true;
  }
}

console.log(
  failed
    ? "\nFAIL: fix the packs above before shipping an app that has no built-in skins."
    : "\nOK: local packs are consistent; deploy index.json + the zips together before releasing the unbundled app.",
);
process.exit(failed ? 1 : 0);

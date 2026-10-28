import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const i18n = readFileSync(new URL("../src/i18n.ts", import.meta.url), "utf8");
const rust = readFileSync(new URL("../src-tauri/src/share_image.rs", import.meta.url), "utf8");

test("share buttons open a preview instead of copying silently", () => {
  assert.match(main, /async function renderElementSnapshot\(el: HTMLElement, W: number\): Promise<string>/);
  assert.match(main, /function openSharePreview\(/);
  assert.match(main, /openSharePreview\(clone, W,/);
  assert.match(main, /openSharePreview\(card, W,/);
  assert.doesNotMatch(main, /copyElementSnapshot\(/);
  assert.match(main, /data-share="__total__"[^>]*aria-haspopup="menu"/);
  assert.match(main, /data-share="\$\{shown\.id\}"[^>]*aria-haspopup="dialog"/);
});

test("share preview exposes copy and save with visible result text", () => {
  assert.match(main, /share-preview-result/);
  assert.match(main, /copyShareDataUrl\(dataUrl\)/);
  assert.match(main, /invoke<string>\("save_share_image"/);
  assert.match(main, /share\.copied/);
  assert.match(main, /share\.saved/);
  assert.match(main, /share\.saveFailed/);
  assert.match(main, /share\.renderFailed/);
});

test("share preview strings exist in all three languages", () => {
  for (const key of [
    "share.previewSubtitle",
    "share.generating",
    "share.copy",
    "share.save",
    "share.close",
    "share.copied",
    "share.copyFailed",
    "share.saved",
    "share.renderFailed",
    "share.saveFailed",
  ]) {
    const hits = i18n.match(new RegExp(`"${key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"`, "g"))?.length ?? 0;
    assert.equal(hits, 3, `${key} must exist in en/zh/ru`);
  }
});

test("share file names are sanitized and never overwrite", () => {
  assert.match(rust, /pub fn sanitize_file_name/);
  assert.match(rust, /pub fn unique_path/);
  assert.match(rust, /sanitize_file_name\("\.\.\/\.\.\/etc\/passwd"\)/);
  assert.match(rust, /unique_path\(&dir, "pane\.png"\)/);
});

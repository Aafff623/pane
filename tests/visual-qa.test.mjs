// #104-10 — the visual acceptance matrix is generated instead of remembered.
// The release gate asks for three skins, both themes and widths from 380 to
// 1440; a paragraph of instructions loses combinations, a table does not.
import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { coverage, skinMatrix, languageMatrix } from "../scripts/visual-qa.mjs";

const script = fileURLToPath(new URL("../scripts/visual-qa.mjs", import.meta.url));

test("the visual matrix covers the release gate's bounds", () => {
  const c = coverage();
  assert.ok(c.skins.includes("native"), "the built-in look is part of the gate");
  for (const skin of ["signal", "ink", "comic-pixel"]) assert.ok(c.skins.includes(skin), skin);
  assert.deepEqual(c.themes.sort(), ["dark", "light"]);
  assert.equal(c.minWidth, 380);
  assert.equal(c.maxWidth, 1440);
  assert.equal(c.languages.length, 3, "zh / en / ru");
});

test("every skin is crossed with both themes at every width", () => {
  const rows = skinMatrix();
  const seen = new Set(rows.map((r) => `${r.skin}|${r.theme}|${r.width}`));
  assert.equal(seen.size, rows.length, "no duplicate combination");
  for (const skin of coverage().skins) {
    for (const theme of ["light", "dark"]) {
      for (const width of [380, 768, 1024, 1440]) {
        assert.ok(seen.has(`${skin}|${theme}|${width}`), `${skin} ${theme} ${width}`);
      }
    }
  }
});

test("the non-happy states are named, not implied", () => {
  const rows = languageMatrix();
  for (const state of ["empty", "partial", "error"]) {
    assert.ok(rows.some((r) => r.state === state), `${state} must be a row`);
  }
  // The empty state's expectation is the one that has bitten us: a 0 is a
  // measurement, an empty account has none.
  assert.match(rows.find((r) => r.state === "empty").expect, /绝不显示 0/);
});

test("--check passes and the generated markdown lists every row", () => {
  const ok = execFileSync(process.execPath, [script, "--check"], { encoding: "utf8" });
  assert.match(ok, /checks cover 4 皮肤 × 2 主题 × 4 宽度 \+ 3 语言 × 4 状态/);
  const md = execFileSync(process.execPath, [script], { encoding: "utf8" });
  for (const heading of ["## 1. 皮肤 × 明暗 × 宽度", "## 2. 语言 × 非正常态", "## 3. 每项都要看的三件事"]) {
    assert.ok(md.includes(heading), heading);
  }
  assert.equal((md.match(/\| ☐ \|/g) || []).length, skinMatrix().length + languageMatrix().length);
});

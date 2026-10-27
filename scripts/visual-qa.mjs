#!/usr/bin/env node
// Visual QA harness (#104-10), zero dependencies.
//
// The spec's release gate asks the owner to sign off three skins, both themes
// and widths from 380 to 1440. Doing that from memory means skipping
// combinations, so this prints the whole matrix as a checklist to tick through
// instead of a paragraph of instructions.
//
// It deliberately does NOT drive a browser or run in CI: the states below need
// a live account, a disconnected network and a real window, so the harness
// produces the plan and the evidence stays human. Run it and paste the table
// into the acceptance issue.
//
//   node scripts/visual-qa.mjs              # markdown checklist on stdout
//   node scripts/visual-qa.mjs --out qa.md  # write it to a file
//   node scripts/visual-qa.mjs --json       # the matrix as data (tests use this)
//   node scripts/visual-qa.mjs --check      # fail if the matrix lost coverage

const LANGUAGES = [
  { id: "zh", label: "中文" },
  { id: "en", label: "English" },
  { id: "ru", label: "Русский" },
];

// 380 is the narrowest supported window and 1440 the widest the panel is
// expected on; the two in between are the breakpoints where the card grid
// changes column count.
const WIDTHS = [380, 768, 1024, 1440];

const THEMES = [
  { id: "light", label: "浅色" },
  { id: "dark", label: "深色" },
];

// `native` is the built-in look (no pack installed); the other three ship as
// resource packs, so this also proves the pack path renders outside the market.
const SKINS = [
  { id: "native", label: "原生" },
  { id: "signal", label: "云端邮局" },
  { id: "ink", label: "山水笺" },
  { id: "comic-pixel", label: "像素分镜" },
];

// The four states a card can be in when the numbers are not simply "fine".
// Each one has burned us before: an empty account rendering 0, a partial sweep
// looking complete, an error looking like zero usage.
const STATES = [
  { id: "populated", label: "有数据（正常账号）", expect: "数字与官网一致" },
  { id: "empty", label: "无账号 / 空态", expect: "显示空态，绝不显示 0" },
  { id: "partial", label: "部分数据（扫描截断）", expect: "标注 partial，不冒充完整" },
  { id: "error", label: "错误态（断网 / 凭据失效）", expect: "红色错误，不显示 0" },
];

export function skinMatrix() {
  const rows = [];
  for (const skin of SKINS)
    for (const theme of THEMES)
      for (const width of WIDTHS)
        rows.push({ skin: skin.id, skinLabel: skin.label, theme: theme.id, themeLabel: theme.label, width });
  return rows;
}

export function languageMatrix() {
  const rows = [];
  for (const lang of LANGUAGES) for (const state of STATES) rows.push({ lang: lang.id, langLabel: lang.label, state: state.id, stateLabel: state.label, expect: state.expect });
  return rows;
}

export function coverage() {
  const skins = new Set(SKINS.map((s) => s.id));
  const themes = new Set(THEMES.map((t) => t.id));
  const langs = new Set(LANGUAGES.map((l) => l.id));
  return {
    skins: [...skins],
    themes: [...themes],
    languages: [...langs],
    widths: [...WIDTHS],
    states: STATES.map((s) => s.id),
    skinRows: skinMatrix().length,
    stateRows: languageMatrix().length,
    minWidth: Math.min(...WIDTHS),
    maxWidth: Math.max(...WIDTHS),
  };
}

function markdown() {
  const skins = skinMatrix();
  const states = languageMatrix();
  const out = [];
  out.push("# 视觉验收清单（自动生成）", "");
  out.push(`生成命令：\`node scripts/visual-qa.mjs\` · 共 ${skins.length + states.length} 项`, "");
  out.push("勾选规则：**逐项真机看过才打勾**。任何一项不通过，就在备注里写清楚是皮肤、主题、宽度还是语言出的问题，不要合并成一句「有点问题」。", "");
  out.push(`## 1. 皮肤 × 明暗 × 宽度（${skins.length} 项）`, "");
  out.push("| # | 皮肤 | 主题 | 宽度 | 通过 | 备注 |", "| --- | --- | --- | --- | --- | --- |");
  skins.forEach((row, i) => out.push(`| ${i + 1} | ${row.skinLabel} | ${row.themeLabel} | ${row.width}px | ☐ | |`));
  out.push("");
  out.push(`## 2. 语言 × 非正常态（${states.length} 项）`, "");
  out.push("| # | 语言 | 状态 | 期望 | 通过 | 备注 |", "| --- | --- | --- | --- | --- | --- |");
  states.forEach((row, i) => out.push(`| ${i + 1} | ${row.langLabel} | ${row.stateLabel} | ${row.expect} | ☐ | |`));
  out.push("");
  out.push("## 3. 每项都要看的三件事", "");
  out.push("- 文字：没有截断、没有叠字、没有溢出到相邻列；数字与单位之间不断行。");
  out.push("- 布局：卡片列数随宽度变化时不留孤儿卡；底部不被托盘/标题栏压住。");
  out.push("- 交互：Tab 走遍可操作项且焦点可见；Esc 能关掉当前浮层。");
  return out.join("\n");
}

const args = process.argv.slice(2);
if (args.includes("--json")) {
  console.log(JSON.stringify({ skins: skinMatrix(), states: languageMatrix(), coverage: coverage() }, null, 2));
} else if (args.includes("--check")) {
  const c = coverage();
  const problems = [];
  if (c.skins.length < 4) problems.push(`皮肤只覆盖了 ${c.skins.length} 种：${c.skins.join(", ")}`);
  if (c.themes.length !== 2) problems.push(`主题应有 2 种，实际 ${c.themes.join(", ")}`);
  if (c.languages.length < 3) problems.push(`语言只覆盖了 ${c.languages.length} 种`);
  if (c.minWidth > 380) problems.push(`最窄宽度 ${c.minWidth} > 380`);
  if (c.maxWidth < 1440) problems.push(`最宽宽度 ${c.maxWidth} < 1440`);
  if (c.states.length < 4) problems.push(`异常态只覆盖了 ${c.states.length} 种`);
  if (problems.length) {
    for (const problem of problems) console.error(`visual-qa: ${problem}`);
    process.exit(1);
  }
  console.log(`visual-qa: ${c.skinRows + c.stateRows} checks cover ${c.skins.length} 皮肤 × 2 主题 × ${c.widths.length} 宽度 + ${c.languages.length} 语言 × ${c.states.length} 状态`);
} else {
  const outIndex = args.indexOf("--out");
  const body = markdown();
  if (outIndex >= 0 && args[outIndex + 1]) {
    const { writeFileSync } = await import("node:fs");
    writeFileSync(args[outIndex + 1], body + "\n", "utf8");
    console.log(`written ${args[outIndex + 1]}`);
  } else {
    console.log(body);
  }
}

// #104-17 — the usage-updated broadcast used to be a bare Snapshot[]: every
// open window adopted every delivery, including the echo of a refresh it had
// itself triggered and already applied from the invoke return value. On the
// panel that second render could land on top of an in-flight card interaction.
// The payload now names the window that caused the refresh, and that window
// skips its own echo.
//
// This is a source contract rather than a behavioural test: the listener lives
// inside main.ts's single top-level function and the emitter inside the Tauri
// app handle, so neither can be imported and driven under node --test. The
// assertions below pin the three things that would silently break it —
// the envelope shape, the caller-reported label, and the skip.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const main = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');
const lib = readFileSync(new URL('../src-tauri/src/lib.rs', import.meta.url), 'utf8');

test('usage-updated names its origin window (#104-17)', () => {
  assert.match(lib, /struct UsageUpdate \{/, 'the broadcast carries a named envelope');
  assert.match(lib, /origin: String/, 'the envelope names a window');
  assert.match(lib, /at: u64/, 'delivery time is recorded for out-of-order diagnosis');
  assert.match(
    lib,
    /fn emit_usage_coalesced\(app: &tauri::AppHandle, origin: &str, snapshots: &\[providers::Snapshot\]\)/,
    'origin is a required argument, so no call site can omit it',
  );
  // The coalescer must keep the origin alongside the snapshots: burst collapses
  // to the LAST payload, and the last origin is the one whose window most needs
  // skipping.
  assert.match(lib, /Mutex<Option<\(String, Vec<providers::Snapshot>\)>>/);
  assert.match(lib, /if let Some\(\(origin, snapshots\)\) = payload/);
});

test('fetch_usage reports its own caller, the backend path names none (#104-17)', () => {
  const command = lib.slice(
    lib.indexOf('async fn fetch_usage('),
    lib.indexOf('/// Refreshes ONE provider card'),
  );
  assert.match(command, /window: tauri::WebviewWindow/, 'the calling window is injected by Tauri');
  assert.match(
    command,
    /emit_usage_coalesced\(&app, window\.label\(\), &snapshots\)/,
    'the broadcast names the real caller rather than a hard-coded label',
  );
  assert.match(
    lib,
    /emit_usage_coalesced\(&app, "backend", &snapshots\)/,
    'the timer and tray pass has no webview, so no window may skip it',
  );
});

test('a window skips its own echo but still adopts a bare array (#104-17)', () => {
  const listener = main.slice(
    main.indexOf('interface UsageUpdate {'),
    main.indexOf('// Back from hidden:'),
  );
  assert.ok(listener.length > 200, 'the listener was found');
  assert.match(listener, /interface UsageUpdate \{ origin\?: string; at\?: number; snapshots: Snapshot\[\] \}/);
  assert.match(listener, /Array\.isArray\(e\.payload\)/, 'an array-only payload is still adopted');
  assert.match(listener, /update\.origin === WINDOW_LABEL/, 'the caller skips its own refresh echo');
  assert.match(listener, /if \(update\.origin === WINDOW_LABEL\) return;/, 'the skip happens before any render work');
  // The skip must not swallow every payload: the guard is equality, never truthiness.
  assert.ok(!/if \(update\.origin\) return;/.test(listener), 'a named origin from another window still applies');
});

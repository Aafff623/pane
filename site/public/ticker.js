// One rAF loop for the whole page (#87). Every self-scheduling animation used
// to own its own loop — the demo annotations, the collector beam, the count-up
// figures — so a single visit could keep four loops alive at once. Subscribers
// now ask for a frame rate and get throttled callbacks from one loop, which
// stops entirely when nobody is subscribed or the tab is hidden.
//
// Written to be importable outside a browser (the site tests stub the globals),
// so every host API is read lazily through globalThis.

const subscribers = new Set();
let frame = 0;
let running = false;

const raf = (cb) =>
  globalThis.requestAnimationFrame
    ? globalThis.requestAnimationFrame(cb)
    : setTimeout(() => cb(Date.now()), 16);
const cancel = (id) =>
  globalThis.cancelAnimationFrame ? globalThis.cancelAnimationFrame(id) : clearTimeout(id);
const hidden = () => Boolean(globalThis.document?.hidden);

function pump(now) {
  frame = 0;
  // Copy before iterating: a callback may unsubscribe itself or another
  // subscriber while the loop is running.
  for (const sub of [...subscribers]) {
    if (now - sub.last >= sub.interval) {
      sub.last = now;
      sub.fn(now);
    }
  }
  if (subscribers.size && !hidden()) {
    frame = raf(pump);
    running = true;
  } else {
    running = false;
  }
}

function start() {
  if (running || !subscribers.size || hidden()) return;
  running = true;
  frame = raf(pump);
}

function stop() {
  if (frame) cancel(frame);
  frame = 0;
  running = false;
}

if (globalThis.document?.addEventListener) {
  globalThis.document.addEventListener("visibilitychange", () => (hidden() ? stop() : start()));
}

/// Subscribe to the shared loop. `fps` caps this subscriber's callback rate
/// (60 = every frame, 30 = every other frame on a 60 Hz display). Returns an
/// unsubscribe function; the loop stops when the last subscriber leaves.
export function subscribe(fn, fps = 60) {
  // The 1 ms tolerance absorbs rAF timestamp jitter: without it a 60 fps
  // subscriber on a 60 Hz display lands fractionally inside its own interval
  // every other frame and silently runs at half rate.
  const interval = fps > 0 && Number.isFinite(fps) ? Math.max(0, 1000 / fps - 1) : 0;
  const sub = { fn, interval, last: -Infinity };
  subscribers.add(sub);
  start();
  return () => {
    if (!subscribers.delete(sub)) return;
    if (!subscribers.size) stop();
  };
}

/// Test/inspection hook: how many subscribers the loop is carrying and
/// whether it is currently scheduled.
export function tickerState() {
  return { subscribers: subscribers.size, running };
}

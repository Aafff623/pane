// MOTDES TargetCursor, owner-supplied 2026-10-16. Native lifecycle port for
// the website Demo only; the desktop frontend is unchanged.
const CORNER_ANGLES = [0, 90, 180, 270];
const QUARTER = Math.PI / 2;

const spring = (state, goal, stiffness, damping, dt) => {
  // Keep integration stable when the Demo timer shim delivers a slow frame.
  const steps = Math.max(1, Math.ceil(dt * 120));
  const h = dt / steps;
  for (let i = 0; i < steps; i++) {
    state.v += (stiffness * (goal - state.x) - damping * state.v) * h;
    state.x += state.v * h;
  }
};


function mountTargetCursor() {
  const settings = {
    targetSelector: 'button, a[href], [role="button"], [role="tab"], [data-jump-provider], .provider',
    cursorColor: '#ffffff', cursorColorOnTarget: undefined,
    spinDuration: 5, hoverDuration: 0.24, parallaxOn: true,
    hideDefaultCursor: true, size: 24, cornerSize: 8, thickness: 1.5,
    padding: 4, matchRadius: true, dotSize: 3, showLabel: false, clickEffect: true,
  };
  const root = document.createElement('div');
  root.className = 'target-cursor';
  root.setAttribute('aria-hidden', 'true');
  root.style.mixBlendMode = 'difference';
  root.style.opacity = '0';
  root.innerHTML = '<span class="target-cursor__corner"></span>'.repeat(4)
    + '<span class="target-cursor__dot"></span><span class="target-cursor__label"></span>';
  document.body.append(root);
  const corners = [...root.querySelectorAll('.target-cursor__corner')];
  const dotElement = root.querySelector('.target-cursor__dot');
  const labelElement = root.querySelector('.target-cursor__label');
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const style = document.createElement('style');
  document.head.appendChild(style);

  const pointer = { x: 0, y: 0, inside: false, seen: false, pressed: false };
  const centerX = { x: 0, v: 0 };
  const centerY = { x: 0, v: 0 };
  const halfW = { x: 0, v: 0 };
  const halfH = { x: 0, v: 0 };
  const radius = { x: 0, v: 0 };
  const turn = { x: 0, v: 0 };
  const lock = { x: 0, v: 0 };
  const press = { x: 0, v: 0 };
  const presence = { x: 0, v: 0 };
  let target = null;
  let targetRadius = 0;
  let quarter = 0;
  let spinSpeed = 0;
  let raf = 0;
  let last = 0;
  let lastLabel = '';
  let lastColor = '';
  let lastScope = null;

  const scopeOf = () => document.documentElement;

  const hideCursor = () => {
    const s = settings;
    const scope = scopeOf();
    if (scope !== lastScope) {
      lastScope?.removeAttribute('data-target-cursor-scope');
      scope?.setAttribute('data-target-cursor-scope', '');
      lastScope = scope;
    }
    const selector = scope ? '[data-target-cursor-scope]' : 'html';
    const rule =
      s.hideDefaultCursor && pointer.inside ? `${selector}, ${selector} * { cursor: none !important; }` : '';
    if (style.textContent !== rule) style.textContent = rule;
  };

  const findTarget = (x, y, element) => {
    const s = settings;
    const node = element || document.elementFromPoint(x, y);
    const found = node?.closest?.(s.targetSelector) || null;
    if (found?.matches(':disabled, [aria-disabled="true"]')) return null;
    const scope = scopeOf();
    if (found && scope && !scope.contains(found)) return null;
    return found;
  };

  const lockOn = next => {
    if (next === target) return;
    target = next;
    if (target) {
      const angle = turn.x;
      quarter = Math.round(angle / QUARTER);
      const corner = parseFloat(getComputedStyle(target).borderTopLeftRadius) || 0;
      targetRadius = corner;
      spinSpeed = 0;
    }
  };

  const frame = now => {
    raf = 0;
    const s = settings;
    const dt = last ? Math.min(0.05, (now - last) / 1000) : 1 / 60;
    last = now;

    if (target && !target.isConnected) lockOn(findTarget(pointer.x, pointer.y));
    const rect = target ? target.getBoundingClientRect() : null;
    const locked = !!rect && pointer.inside;
    const snapW = 4.6 / s.hoverDuration;
    const snapK = snapW * snapW;
    const snapC = 2 * 0.62 * snapW;

    spring(presence, pointer.inside ? 1 : 0, 260, 32, dt);
    spring(lock, locked ? 1 : 0, snapK, 2 * snapW, dt);
    spring(press, pointer.pressed && s.clickEffect ? 1 : 0, 900, 40, dt);

    let goalX = pointer.x;
    let goalY = pointer.y;
    let goalW = s.size / 2;
    let goalH = s.size / 2;
    let goalR = Math.min(2, s.cornerSize / 4);
    if (locked) {
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const lean = s.parallaxOn ? 0.08 : 0;
      goalX = cx + (pointer.x - cx) * lean;
      goalY = cy + (pointer.y - cy) * lean;
      const odd = Math.abs(quarter) % 2 === 1;
      const w = rect.width / 2 + s.padding;
      const h = rect.height / 2 + s.padding;
      goalW = odd ? h : w;
      goalH = odd ? w : h;
      goalR = s.matchRadius ? Math.min(targetRadius + Math.max(0, s.padding), s.cornerSize) : 0;
    }

    if (!pointer.inside && presence.x < 0.01) pointer.seen = false;
    if (!pointer.seen) {
      centerX.x = goalX;
      centerY.x = goalY;
      halfW.x = goalW;
      halfH.x = goalH;
      pointer.seen = pointer.inside;
    }

    const follow = locked ? snapK : 2200;
    const followC = locked ? snapC : 2 * Math.sqrt(2200);
    spring(centerX, goalX, follow, followC, dt);
    spring(centerY, goalY, follow, followC, dt);
    spring(halfW, goalW, snapK, snapC, dt);
    spring(halfH, goalH, snapK, snapC, dt);
    spring(radius, goalR, snapK, 2 * snapW, dt);

    if (locked) {
      spring(turn, quarter * QUARTER, snapK, snapC, dt);
    } else {
      const speed = s.spinDuration > 0 && !reduce ? (Math.PI * 2) / s.spinDuration : 0;
      spinSpeed += (speed - spinSpeed) * (1 - Math.exp(-dt * 3));
      turn.v = spinSpeed;
      turn.x += spinSpeed * dt;
    }

    const squeeze = 1 - press.x * 0.14;
    const cos = Math.cos(turn.x);
    const sin = Math.sin(turn.x);
    const w = Math.max(0, halfW.x) * squeeze;
    const h = Math.max(0, halfH.x) * squeeze;
    const local = [
      [-w, -h],
      [w, -h],
      [w, h],
      [-w, h]
    ];
    const r = Math.max(0, Math.min(radius.x, s.cornerSize));
    const degrees = (turn.x * 180) / Math.PI;
    let left = Infinity;
    let top = Infinity;
    corners.forEach((corner, i) => {
      if (!corner) return;
      const [lx, ly] = local[i];
      const x = centerX.x + lx * cos - ly * sin;
      const y = centerY.x + lx * sin + ly * cos;
      left = Math.min(left, x);
      top = Math.min(top, y);
      corner.style.transform = `translate3d(${x.toFixed(2)}px, ${y.toFixed(2)}px, 0) rotate(${(CORNER_ANGLES[i] + degrees).toFixed(2)}deg)`;
      corner.style.width = `${s.cornerSize}px`;
      corner.style.height = `${s.cornerSize}px`;
      corner.style.borderWidth = `${s.thickness}px 0 0 ${s.thickness}px`;
      corner.style.borderTopLeftRadius = `${r.toFixed(2)}px`;
    });

    const dot = dotElement;
    if (dot) {
      const dotScale = 1 - press.x * 0.35;
      dot.style.transform = `translate3d(${pointer.x}px, ${pointer.y}px, 0) translate(-50%, -50%) scale(${dotScale.toFixed(3)})`;
      dot.style.width = `${s.dotSize}px`;
      dot.style.height = `${s.dotSize}px`;
    }

    const label = labelElement;
    if (label) {
      const text = s.showLabel && target ? target.getAttribute('data-cursor-label') || '' : '';
      if (text && text !== lastLabel) label.textContent = text;
      if (text) lastLabel = text;
      const shown = text ? Math.max(0, Math.min(1, lock.x)) : 0;
      label.style.transform = `translate3d(${left.toFixed(2)}px, ${(top - 8).toFixed(2)}px, 0) translateY(-100%) translateY(${((1 - shown) * 4).toFixed(2)}px)`;
      label.style.opacity = shown.toFixed(3);
    }

    const mix = s.cursorColorOnTarget ? Math.round(Math.max(0, Math.min(1, lock.x)) * 100) : 0;
    const color = mix ? `color-mix(in oklab, ${s.cursorColorOnTarget} ${mix}%, ${s.cursorColor})` : s.cursorColor;
    if (color !== lastColor) {
      root.style.setProperty('--target-cursor-color', color);
      lastColor = color;
    }
    root.style.opacity = Math.max(0, Math.min(1, presence.x)).toFixed(3);

    const resting =
      !pointer.inside &&
      presence.x < 0.002 &&
      Math.abs(presence.v) < 0.002 &&
      Math.abs(lock.x) < 0.002 &&
      Math.abs(press.x) < 0.002;
    const spinning = pointer.inside && !locked && spinSpeed > 0.0001;
    const moving =
      Math.abs(centerX.v) + Math.abs(centerY.v) + Math.abs(halfW.v) + Math.abs(halfH.v) + Math.abs(turn.v) > 0.01 ||
      Math.abs(goalX - centerX.x) + Math.abs(goalY - centerY.x) > 0.05 ||
      Math.abs(lock.v) + Math.abs(press.v) + Math.abs(presence.v) > 0.001;
    if (!resting && (spinning || moving || locked)) raf = requestAnimationFrame(frame);
    else last = 0;
  };

  const wake = () => {
    if (!raf) raf = requestAnimationFrame(frame);
  };


  const insideScope = (x, y, element) => {
    const scope = scopeOf();
    if (!scope) return true;
    if (element && scope.contains(element)) return true;
    const box = scope.getBoundingClientRect();
    return x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
  };

  const onPointerMove = event => {
    if (event.pointerType && event.pointerType !== 'mouse') return;
    if (event.target.closest?.('input, textarea, select, [contenteditable="true"]')) { onLeave(); return; }
    pointer.x = event.clientX;
    pointer.y = event.clientY;
    const inside = insideScope(pointer.x, pointer.y, event.target);
    if (inside !== pointer.inside) {
      pointer.inside = inside;
      hideCursor();
    }
    lockOn(inside ? findTarget(pointer.x, pointer.y, event.target) : null);
    wake();
  };

  const onPointerDown = event => {
    if (event.pointerType && event.pointerType !== 'mouse') return;
    pointer.pressed = true;
    wake();
  };

  const onPointerUp = () => {
    pointer.pressed = false;
    wake();
  };

  const onLeave = () => {
    pointer.inside = false;
    pointer.pressed = false;
    lockOn(null);
    hideCursor();
    wake();
  };

  const onScroll = () => {
    if (!pointer.inside) return;
    lockOn(findTarget(pointer.x, pointer.y));
    wake();
  };
  const onKeyDown = event => {
    if (event.key === 'Tab' || event.key === 'Escape') onLeave();
  };

  window.addEventListener('pointermove', onPointerMove, { passive: true });
  window.addEventListener('pointerdown', onPointerDown, { passive: true });
  window.addEventListener('pointerup', onPointerUp, { passive: true });
  window.addEventListener('scroll', onScroll, { passive: true, capture: true });
  document.documentElement.addEventListener('pointerleave', onLeave);
  window.addEventListener('blur', onLeave);
  window.addEventListener('pointercancel', onLeave);
  window.addEventListener('keydown', onKeyDown);

  return () => {
    cancelAnimationFrame(raf);
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerdown', onPointerDown);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('scroll', onScroll, { capture: true });
    document.documentElement.removeEventListener('pointerleave', onLeave);
    window.removeEventListener('blur', onLeave);
    window.removeEventListener('pointercancel', onLeave);
    window.removeEventListener('keydown', onKeyDown);
    root.remove();
    lastScope?.removeAttribute('data-target-cursor-scope');
    style.remove();
  };
}

const fine = matchMedia('(hover: hover) and (pointer: fine)');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let paused = false;
try { paused = localStorage.getItem('pane-site-motion') === 'paused' || localStorage.getItem('pane-demo-target-cursor') === 'off'; } catch {}
let dispose;
const sync = () => {
  const active = fine.matches && !reduced.matches && !paused && !document.hidden;
  if (active && !dispose) dispose = mountTargetCursor();
  else if (!active && dispose) { dispose(); dispose = undefined; }
};
window.addEventListener('message', event => {
  if (event.origin !== location.origin || event.source !== parent || event.data?.type !== 'pane-demo-motion') return;
  paused = !!event.data.paused;
  sync();
});
fine.addEventListener('change', sync);
reduced.addEventListener('change', sync);
document.addEventListener('visibilitychange', sync);
window.addEventListener('pagehide', () => { dispose?.(); dispose = undefined; });
window.addEventListener('pageshow', sync);
sync();

// Accumulate direction changes so tiny trackpad movements do not flicker the bar.
export function createHeaderTracker(initialY = 0, top = 64) {
  let previous = Math.max(0, initialY), direction = 0, travel = 0;
  let hidden = previous > top;
  return rawY => {
    const y = Math.max(0, rawY);
    const delta = y - previous;
    previous = y;
    if (y <= top) { hidden = false; direction = 0; travel = 0; }
    else if (delta !== 0) {
      const nextDirection = Math.sign(delta);
      if (nextDirection !== direction) { direction = nextDirection; travel = 0; }
      travel += Math.abs(delta);
      if (travel >= 10) hidden = direction > 0;
    }
    return hidden;
  };
}

export function initHeaderNav() {
  const header = document.querySelector('.site-header');
  if (!header) return;
  const readY = () => Math.min(Math.max(0, scrollY), Math.max(0, document.documentElement.scrollHeight - innerHeight));
  const track = createHeaderTracker(readY(), header.offsetHeight);
  let frame = 0;
  const sync = () => { frame = 0; header.classList.toggle('is-hidden', track(readY())); };
  const onScroll = () => { if (!frame) frame = requestAnimationFrame(sync); };
  window.addEventListener('scroll', onScroll, { passive:true });
  window.addEventListener('pageshow', sync);
  sync();
  window.addEventListener('pagehide', event => {
    cancelAnimationFrame(frame); frame = 0;
    if (event.persisted) return;
    window.removeEventListener('scroll', onScroll);
    window.removeEventListener('pageshow', sync);
  });
}

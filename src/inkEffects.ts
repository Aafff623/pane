export function decorateInkSkin(host: HTMLElement): void {
    if (!document.getElementById('ink-graph-paint')) {
        const defs = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        defs.setAttribute('aria-hidden', 'true');
        defs.setAttribute('width', '0');
        defs.setAttribute('height', '0');
        defs.style.position = 'absolute';
        defs.style.pointerEvents = 'none';
        defs.innerHTML = '<defs><filter id="ink-pigment" x="-4%" y="-8%" width="108%" height="116%"><feTurbulence type="fractalNoise" baseFrequency=".085 .22" numOctaves="2" seed="7" result="grain"/><feDisplacementMap in="SourceGraphic" in2="grain" scale="1.5" xChannelSelector="R" yChannelSelector="G"/></filter></defs>';
        const graphDefs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
        graphDefs.innerHTML = `
   <filter id="ink-graph-paint" x="-12%" y="-12%" width="124%" height="124%">
    <feTurbulence type="fractalNoise" baseFrequency=".45 .18" numOctaves="2" seed="19" result="fiber"/>
    <feColorMatrix in="fiber" type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 1 0 0 0 0"/>
    <feComponentTransfer><feFuncA type="discrete" tableValues=".55 .8 1 1"/></feComponentTransfer>
    <feComposite in="SourceGraphic" operator="in" result="dryBrush"/>
    <feDisplacementMap in="dryBrush" in2="fiber" scale=".65" xChannelSelector="R" yChannelSelector="G"/>
   </filter>
   <linearGradient id="ink-column-wash" x1="0%" y1="0%" x2="0%" y2="100%">
    <stop offset="0" stop-color="var(--ink-stroke)" stop-opacity="1"/>
    <stop offset=".2" stop-color="var(--ink-stroke)" stop-opacity=".9"/>
    <stop offset=".55" stop-color="var(--ink-stroke)" stop-opacity=".55"/>
    <stop offset="1" stop-color="var(--ink-stroke)" stop-opacity=".16"/>
   </linearGradient>
   <pattern id="ink-column-fibers" width="7" height="13" patternUnits="userSpaceOnUse">
    <path d="M1 1v5m3 2v4m2-11v2" stroke="var(--card)" stroke-width=".5" opacity=".5"/>
   </pattern>`;
        defs.append(graphDefs);
        document.body.append(defs);
    }
    if (!document.documentElement.hasAttribute('data-ink'))
        return;
    for (const chart of host.querySelectorAll('.donut-wrap')) {
        if (chart.querySelector('.ink-donut-art'))
            continue;
        const ring = chart.querySelector(':scope > svg');
        if (!ring)
            continue;
        const art = document.createElement('div');
        art.className = 'ink-donut-art';
        const fish = document.createElement('span');
        fish.className = 'ink-koi';
        fish.setAttribute('aria-hidden', 'true');
        ring.before(art);
        art.append(fish, ring);
    }
    for (const bar of host.querySelectorAll('.trend-chart .trend-bar')) {
        if (bar.nextElementSibling?.classList.contains('ink-column-fiber'))
            continue;
        const fibers = bar.cloneNode(false) as Element;
        fibers.setAttribute('class', 'ink-column-fiber');
        fibers.setAttribute('aria-hidden', 'true');
        bar.after(fibers);
    }
}
export function clearInkSkin(host: HTMLElement): void {
    for (const art of host.querySelectorAll('.ink-donut-art')) {
        const ring = art.querySelector('svg');
        if (ring)
            art.replaceWith(ring);
        else
            art.remove();
    }
    host.querySelectorAll('.ink-column-fiber').forEach(node => node.remove());
}

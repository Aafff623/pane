import { gsap } from 'gsap';
import type { SkinId } from './skins';
import './themeStartup.css';
export function playThemeStartup(skin: SkinId, reduced: boolean, labels: {
    name: string;
    tagline: string;
    skip: string;
}): () => void {
    const theme = skin === 'ink' ? 'ink' : 'post';
    let overlay: HTMLElement | null = null, timer: ReturnType<typeof setTimeout> | undefined, timeline: gsap.core.Timeline | null = null;
    const escape = (e: KeyboardEvent) => { if (e.key === 'Escape' && !e.isComposing) {
        e.preventDefault();
        e.stopImmediatePropagation();
        close();
    } };
    const close = () => { clearTimeout(timer); timeline?.kill(); timeline = null; overlay?.remove(); overlay = null; document.documentElement.removeAttribute('data-theme-intro'); document.removeEventListener('keydown', escape, true); };
    document.documentElement.dataset.themeIntro = theme;
    overlay = document.createElement('section');
    overlay.className = 'theme-intro ' + theme + (reduced ? ' reduced' : '');
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-label', theme === 'ink' ? '山水笺启动动画' : '云端邮局启动动画');
    const art = theme === 'ink' ? `<div class="intro-inkdrop"></div><div class="intro-mountains"></div><div class="intro-koi"></div><div class="intro-ink-seal">山<br>水</div>` :
        `<div class="intro-post-lines"></div><div class="intro-envelope"><div class="intro-letter"><div class="intro-stamp">P</div><span>PANE POST</span><i></i><i></i><i></i></div><div class="intro-flap"></div><div class="intro-pocket"></div><div class="intro-postmark">云端邮局<br><small>用量来信</small></div></div>`;
    overlay.innerHTML = `<div class="intro-art" aria-hidden="true">${art}</div><div class="intro-caption"><span class="intro-brand">PANE</span><h1>${labels.name}</h1><p>${labels.tagline}</p></div><button class="intro-skip" type="button">${labels.skip}</button>`;
    overlay!.querySelector<HTMLButtonElement>('.intro-skip')!.onclick = close;
    document.body.append(overlay);
    overlay!.dataset.engine = 'gsap';
    const find = (s: string) => overlay!.querySelectorAll(s);
    const title = overlay.querySelector('h1');
    title!.innerHTML = [...(title!.textContent || '')].map(c => `<span class="intro-glyph">${c}</span>`).join('');
    const tl = gsap.timeline({ paused: true, defaults: { ease: 'power3.out' }, onComplete: close });
    timeline = tl;
    if (theme === 'ink') {
        tl.fromTo(find('.intro-inkdrop'), { scale: .08, opacity: 0, filter: 'blur(12px)' }, { scale: 2.1, opacity: 1, filter: 'blur(5px)', duration: 1.75, ease: 'power2.out' }, 0)
            .fromTo(find('.intro-mountains'), { y: 24, scale: 1.08, opacity: 0, filter: 'blur(9px)' }, { y: 0, scale: 1, opacity: .23, filter: document.documentElement.dataset.theme === 'dark' ? 'invert(1) blur(0px)' : 'blur(0px)', duration: 2.2, ease: 'sine.out' }, .12)
            .fromTo(find('.intro-koi'), { rotation: -52, scale: .72, opacity: 0, filter: 'blur(5px)' }, { rotation: 0, scale: 1, opacity: 1, filter: 'blur(0px)', duration: 1.9, ease: 'power2.out' }, .3)
            .fromTo(find('.intro-ink-seal'), { scale: 1.2, opacity: 0, y: -5 }, { scale: 1, opacity: 1, y: 0, duration: .45, ease: 'power4.out' }, 1.5);
    }
    else {
        tl.fromTo(find('.intro-post-lines'), { x: -18, opacity: 0 }, { x: 0, opacity: 1, duration: 1.2 }, 0)
            .fromTo(find('.intro-envelope'), { y: 36, rotation: -5, scale: .94, opacity: 0 }, { y: 0, rotation: 0, scale: 1, opacity: 1, duration: .85, ease: 'power3.out' }, 0)
            .fromTo(find('.intro-flap'), { rotationX: 0 }, { rotationX: 180, duration: .7, ease: 'power2.inOut', transformOrigin: '50% 0%' }, .38)
            .fromTo(find('.intro-letter'), { y: 0, rotation: 0 }, { y: -100, rotation: -1.4, duration: 1.05, ease: 'power3.inOut' }, .65)
            .fromTo(find('.intro-letter i'), { scaleX: 0 }, { scaleX: 1, transformOrigin: '0% 50%', duration: .55, stagger: .09 }, 1.2)
            .fromTo(find('.intro-postmark'), { scale: 1.45, rotation: -21, opacity: 0 }, { scale: 1, rotation: -14, opacity: 1, duration: .35, ease: 'power4.in' }, 1.65)
            .to(find('.intro-envelope'), { y: 2, duration: .09, ease: 'power1.in' }, 1.91)
            .to(find('.intro-envelope'), { y: 0, duration: .22, ease: 'power2.out' }, 2);
    }
    tl.fromTo(find('.intro-brand'), { opacity: 0, y: 5 }, { opacity: .65, y: 0, duration: .6 }, 1.05)
        .fromTo(find('.intro-glyph'), { opacity: 0, y: 14, filter: 'blur(4px)' }, { opacity: 1, y: 0, filter: 'blur(0px)', duration: .65, stagger: .09 }, 1.25)
        .fromTo(find('.intro-caption p'), { opacity: 0, y: 6 }, { opacity: 1, y: 0, duration: .7 }, 1.75)
        .addLabel('settled', 2.65)
        .to(find('.intro-art'), { y: -8, scale: 1.015, opacity: 0, duration: .65, ease: 'power2.inOut' }, 3)
        .to(find('.intro-caption'), { y: -4, opacity: 0, duration: .5, ease: 'power2.inOut' }, 3.15)
        .to(overlay, { opacity: 0, duration: .6, ease: 'power2.inOut' }, 3.2);
    if (reduced) {
        tl.seek('settled').pause();
        overlay!.dataset.phase = 'settled';
        timer = setTimeout(close, 1100);
    }
    else {
        overlay!.dataset.phase = 'playing';
        tl.play(0);
        timer = setTimeout(close, 5500);
    }
    overlay!.addEventListener('click', close);
    document.addEventListener('keydown', escape, true);
    return close;
}

"use client";

import { useEffect, useRef, useState } from "react";

import { Pause, Play } from "lucide-react";

import type { Lang } from "@/components/i18n";

const PORTRAIT = "(max-width: 640px)";
const CUTS = {
  en: {
    landscape: { src: "/video/pane-overview.mp4", poster: "/video/pane-overview.webp" },
    portrait: { src: "/video/pane-overview-mobile.mp4", poster: "/video/pane-overview-mobile.webp" },
  },
  zh: {
    landscape: { src: "/video/pane-overview-zh.mp4", poster: "/video/pane-overview-zh.webp" },
    portrait: { src: "/video/pane-overview-mobile-zh.mp4", poster: "/video/pane-overview-mobile-zh.webp" },
  },
} as const;

const LABELS = {
  en: { play: "Play the demo", pause: "Pause the demo" },
  zh: { play: "播放演示动画", pause: "暂停演示动画" },
};

export function HeroFilm({ lang, label, className, buttonClassName }: { lang: Lang; label: string; className?: string; buttonClassName?: string }) {
  const video = useRef<HTMLVideoElement>(null);
  const [portrait, setPortrait] = useState(false);
  const [shows, setShows] = useState<"play" | "pause">("play");
  // The visitor's intent. Swapping `src` reloads the element and drops the
  // playback state without firing `pause`, so intent has to live outside it.
  const wantsPlay = useRef(true);
  const inView = useRef(false);
  const cut = CUTS[lang][portrait ? "portrait" : "landscape"];

  useEffect(() => {
    const query = matchMedia(PORTRAIT);
    // The server render cannot know the viewport, so the first client pass
    // starts from the landscape defaults and corrects itself here. `<source
    // media>` already picked the right cut during parsing; only a later
    // resize needs an explicit reload.
    let current = false;
    const sync = (initial: boolean) => {
      if (query.matches === current) return;
      current = query.matches;
      setPortrait(current);
      if (!initial) video.current?.load();
    };
    sync(true);
    const onQueryChange = () => sync(false);
    query.addEventListener("change", onQueryChange);
    return () => query.removeEventListener("change", onQueryChange);
  }, []);

  useEffect(() => {
    const player = video.current;
    if (!player) return;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const sync = () => setShows(player.paused ? "play" : "pause");
    const maybePlay = () => {
      if (wantsPlay.current && inView.current && !reduced.matches) void player.play().catch(sync);
      else sync();
    };

    const observer = new IntersectionObserver(([entry]) => {
      inView.current = entry.isIntersecting && entry.intersectionRatio >= 0.45;
      if (inView.current) maybePlay();
      else player.pause();
    }, { threshold: [0, 0.45] });

    const stopOnPreferenceChange = () => { if (reduced.matches) player.pause(); };

    player.addEventListener("play", sync);
    player.addEventListener("pause", sync);
    player.addEventListener("loadedmetadata", maybePlay);
    player.addEventListener("emptied", sync);
    observer.observe(player);
    reduced.addEventListener("change", stopOnPreferenceChange);
    return () => {
      player.removeEventListener("play", sync);
      player.removeEventListener("pause", sync);
      player.removeEventListener("loadedmetadata", maybePlay);
      player.removeEventListener("emptied", sync);
      observer.disconnect();
      reduced.removeEventListener("change", stopOnPreferenceChange);
    };
  }, [cut.src]);

  function toggle() {
    const player = video.current;
    if (!player) return;
    wantsPlay.current = player.paused;
    if (player.paused) void player.play().catch(() => {});
    else player.pause();
  }

  return (
    <>
      <video
        ref={video}
        className={className}
        aria-label={label}
        width={portrait ? 720 : 1152}
        height={portrait ? 920 : 560}
        poster={cut.poster}
        preload="metadata"
        muted
        loop
        playsInline
      >
        {/* `src` alone would make every phone fetch the landscape cut first,
            because the breakpoint is only known after hydration. `<source
            media>` lets the browser choose before it downloads anything. */}
        <source media="(min-width: 641px)" src={CUTS[lang].landscape.src} />
        <source media={PORTRAIT} src={CUTS[lang].portrait.src} />
      </video>
      <button type="button" onClick={toggle} className={buttonClassName} aria-label={LABELS[lang][shows]}>
        {shows === "pause" ? <Pause size={13} aria-hidden="true" /> : <Play size={13} aria-hidden="true" />}
      </button>
    </>
  );
}

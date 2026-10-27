import { paneMotion } from '/motion.js?v=2';
import { createClickSpark } from '/effects/click-spark.js?v=1';

export function initHeroReactions() {
  const root = document.querySelector('.hero-reactions');
  if (!root) return;
  const english = document.documentElement.lang === 'en';
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const labels = english
    ? { issues:'Issues · Report and discuss', pr:'PR · View pull requests', star:'Star · Open the GitHub repository', fork:'Fork · Create a repository copy' }
    : { issues:'Issues · 查看问题与反馈', pr:'PR · 查看合并请求', star:'Star · 前往 GitHub 为项目加星', fork:'Fork · 创建仓库副本' };
  root.setAttribute('aria-label', english ? 'GitHub project links' : 'GitHub 项目入口');
  root.querySelectorAll('[data-reaction]').forEach(button => {
    const action = button.dataset.reaction;
    button.setAttribute('aria-label', labels[action]); button.removeAttribute('title');
  });
  const sparks = createClickSpark(root, { isDisabled:() => paneMotion.paused || reduced.matches || document.documentElement.hasAttribute('data-theme-transition') });
  const clear = () => sparks.clear();
  const unsubscribe = paneMotion.subscribe(clear);
  reduced.addEventListener('change', clear);
  document.addEventListener('visibilitychange', clear);
  const observer = new MutationObserver(clear);
  observer.observe(document.documentElement, { attributes:true, attributeFilter:['data-theme-transition'] });
  window.addEventListener('pagehide', event => {
    if (event.persisted) return;
    unsubscribe(); sparks.destroy(); observer.disconnect();
    reduced.removeEventListener('change', clear);
    document.removeEventListener('visibilitychange', clear);
  });
}

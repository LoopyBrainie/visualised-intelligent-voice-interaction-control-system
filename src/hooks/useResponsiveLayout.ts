import { createSignal, onMount, onCleanup } from 'solid-js';

/**
 * 响应式布局 Hook
 * 检测横屏/竖屏切换，提供布局比例计算
 */
export function useResponsiveLayout() {
  const [isPortrait, setIsPortrait] = createSignal(
    window.matchMedia("(max-aspect-ratio: 1/1)").matches
  );

  onMount(() => {
    const mediaQuery = window.matchMedia("(max-aspect-ratio: 1/1)");
    const handler = (e: MediaQueryListEvent) => setIsPortrait(e.matches);
    mediaQuery.addEventListener('change', handler);
    onCleanup(() => mediaQuery.removeEventListener('change', handler));
  });

  return isPortrait;
}
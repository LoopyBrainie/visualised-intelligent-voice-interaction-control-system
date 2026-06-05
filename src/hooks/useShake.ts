import { createSignal, onCleanup } from 'solid-js';

// 错误反馈：触发元素短暂 shake。Apple 风格的衰减振荡，~400ms。
// CSS 端定义 .animate-shake（src/styles/app.css），prefers-reduced-motion 已自动处理。
//
// 使用：
//   const [shaking, triggerShake] = useShake();
//   <div classList={{ 'animate-shake': shaking() }}>...</div>
//   on error → triggerShake();
const SHAKE_DURATION = 400;

export function useShake() {
  const [shaking, setShaking] = createSignal(false);
  let timer: number | undefined;

  const trigger = () => {
    // 重置 → 下帧重启：CSS animation 只在 attach 时跑一次，连发错误需要这样重置
    setShaking(false);
    requestAnimationFrame(() => {
      setShaking(true);
      if (timer !== undefined) clearTimeout(timer);
      timer = window.setTimeout(() => setShaking(false), SHAKE_DURATION);
    });
  };

  onCleanup(() => {
    if (timer !== undefined) clearTimeout(timer);
  });

  return [shaking, trigger] as const;
}

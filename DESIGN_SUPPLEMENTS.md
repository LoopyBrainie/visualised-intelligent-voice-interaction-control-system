# Design System Supplements
**补充日期:** 2026-04-28
**基于:** DESIGN.md (Apple × Morandi Hybrid)

---

## 1. 组件库扩展 (Components not in DESIGN.md)

### 1.1 Toggle Switch (设备开关)
**设计意图:** iOS 风格滑动开关，用于设备启停控制

| 状态 | 轨道颜色 | 滑块位置 |
|------|---------|---------|
| OFF | `bg-dark-surface-4` (#2a2a2d) | left-0.5 |
| ON | `bg-apple-blue` (#0071e3) | left-[22px] |

**实现规范:**
```tsx
<button class={`
  relative w-11 h-6 rounded-full transition-colors duration-200
  ${deviceState().device ? 'bg-apple-blue' : 'bg-dark-surface-4'}
`}>
  <span class={`
    absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm
    transition-transform duration-200
    ${deviceState().device ? 'left-[22px]' : 'left-0.5'}
  `} />
</button>
```

**关键参数:**
- 轨道尺寸: 44px × 24px (w-11 h-6)
- 滑块尺寸: 20px × 20px (w-5 h-5)
- 滑块偏移: OFF 时 left-0.5 (2px), ON 时 left-[22px] (88px)
- 过渡: `transition-colors duration-200` + `transition-transform duration-200`

---

### 1.2 Speed Selector (风扇档位选择器)
**设计意图:** 数字档位选择器，3 档位垂直排列

**实现规范:**
```tsx
{[1, 2, 3].map((speed) => (
  <button class={`
    w-8 h-8 rounded-apple-sm text-xs font-medium transition-colors duration-200
    ${deviceState().fan_speed === speed
      ? 'bg-apple-blue text-white'
      : 'bg-dark-surface-4 text-apple-text-tertiary hover:text-apple-text-secondary'}
  `}>
    {speed}
  </button>
))}
```

**关键参数:**
- 按钮尺寸: 32px × 32px (w-8 h-8)
- 圆角: `rounded-apple-sm` (5px)
- 选中态: `bg-apple-blue` + `text-white`
- 未选中态: `bg-dark-surface-4` + `text-apple-text-tertiary`

---

### 1.3 Temperature / Brightness Control (温度/亮度调节器)
**设计意图:** +/- 按钮配合数字显示的调节控件

**实现规范:**
```tsx
<div class="flex items-center gap-3">
  {/* 减按钮 */}
  <button class="
    w-8 h-8 rounded-apple-sm bg-dark-surface-4
    text-apple-text-secondary hover:text-apple-text-primary
    transition-colors duration-200 flex items-center justify-center
  ">
    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
    </svg>
  </button>

  {/* 数值显示 */}
  <span class="text-lg font-light text-apple-text-primary w-12 text-center">
    {value}°
  </span>

  {/* 加按钮 */}
  <button class="...同上...">
    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
    </svg>
  </button>
</div>
```

---

### 1.4 Brightness Slider (亮度滑块)
**设计意图:** 自定义样式的 Range 输入，配合百分比显示

**实现规范:**
```tsx
<div class="flex items-center gap-3">
  <span class="text-xs text-apple-text-tertiary">{brightness}%</span>
  <input
    type="range"
    min="0"
    max="100"
    class="w-24 h-1 bg-dark-surface-4 rounded-full appearance-none cursor-pointer
      [&::-webkit-slider-thumb]:appearance-none
      [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3
      [&::-webkit-slider-thumb]:rounded-full
      [&::-webkit-slider-thumb]:bg-apple-blue
      [&::-webkit-slider-thumb]:cursor-pointer"
  />
</div>
```

**关键参数:**
- 轨道: 高度 4px, `bg-dark-surface-4`, 圆角 9999px (full)
- 滑块: 12px × 12px (w-3 h-3), `bg-apple-blue`, 圆形
- 宽度: 96px (w-24)

---

### 1.5 Device Status Indicator (设备状态指示器)
**设计意图:** 圆点 + 文字的状态显示

**实现规范:**
```tsx
<div class="flex items-center gap-2">
  <div class={`
    w-2 h-2 rounded-full transition-colors duration-300
    ${isRunning() ? 'bg-green-500 animate-pulse' : 'bg-apple-text-tertiary'}
  `} />
  <span class="text-xs text-apple-text-tertiary">
    {isRunning() ? '运行中' : '已停止'}
  </span>
</div>
```

---

### 1.6 Room Visualization (房间可视化)
**设计意图:** SVG 平面图展示设备位置和状态

**实现规范:**
- 画布尺寸: 400 × 280 (viewBox)
- 背景: `#2a2d35` (深灰蓝)
- 区域: `#32363e` (客厅/卧室)
- 设备图标: 颜色根据状态动态变化
- 发光效果: `drop-shadow` filter 结合 brightness

**发光透明度计算:**
```ts
const getGlowOpacity = (brightness: number) => 0.5 + (brightness / 200);
// 范围: [0.5, 1.0] 线性映射
```

---

### 1.7 Audio Spectrum Visualizer (音频频谱)
**设计意图:** 32 频谱柱状条，模拟音频可视化

**⚠️ 待修复: 渐变使用**
```tsx
// 当前实现 (违反"无渐变"规则):
background: isActive()
  ? `linear-gradient(to top, #0071e3 ${100 - (i % 4) * 20}%, #2997ff)`
  : '#3a3a3c'

// 建议修复为纯色:
background: isActive() ? '#0071e3' : '#3a3a3c'
```

---

## 2. 已知设计不一致

### 2.1 渐变使用
| 位置 | 问题 | 严重程度 |
|------|------|---------|
| `SpectrumPanel.tsx` | 频谱条使用 linear-gradient | 中 |

### 2.2 SVG 内联颜色
| 位置 | 问题 | 严重程度 |
|------|------|---------|
| `VisualizationPanel.tsx` | 设备颜色硬编码 (#fbbf24, #60a5fa 等) | 低 |

**建议:** 将设备状态颜色提取为 design token:
```ts
// tailwind.config.js
colors: {
  // Device status
  'device-light': '#fbbf24',
  'device-fan': '#34d399',
  'device-ac': '#60a5fa',
  'device-curtain': '#a78bfa',
}
```

---

## 3. 动画系统 (未在 DESIGN.md 记录)

### 3.1 Toggle Switch 滑块动画
```css
transition-transform duration-200
/* 从 left-0.5 平滑移动到 left-[22px] */
```

### 3.2 Spectrum 频谱动画
```tsx
// 简单切换实现
setIsActive(prev => !prev);

// 频谱条高度变化
const height = isActive() ? baseHeight + randomVariation : 8;
```

### 3.3 状态指示器脉冲
```tsx
class="bg-green-500 animate-pulse"
// 或
class="bg-apple-blue animate-pulse"
```

---

## 4. 日志级别设计

| 级别 | 颜色 Token | 背景色 | 标签 |
|------|-----------|--------|------|
| ERROR | `text-log-error` (#ff3b30) | `bg-log-error/10` | 错误 |
| WARN | `text-log-warn` (#ff9500) | `bg-log-warn/10` | 警告 |
| INFO | `text-log-info` (#0071e3) | `bg-log-info/10` | 信息 |
| DEBUG | `text-log-debug` (#8e8e93) | `bg-log-debug/10` | 调试 |

---

## 5. 待办事项

- [x] 修复 SpectrumPanel 渐变问题
- [x] 将 SVG 设备颜色提取为 design token
- [ ] 在 DESIGN.md 中补充 WAAPI 动画规范
- [ ] 添加深色模式完整支持 (目前部分实现)

## 6. 已修复问题 (2026-04-28)

### 6.1 SpectrumPanel 渐变 → 纯色
**文件:** `src/components/SpectrumPanel.tsx`
**变更:** 移除 `linear-gradient`，改用纯 `#0071e3`

### 6.2 SVG 设备颜色 Token 化
**文件:** `tailwind.config.js` + `src/components/VisualizationPanel.tsx`
**变更:**
- 新增 `device-light`, `device-fan`, `device-ac`, `device-curtain` 到 tailwind.config.js
- VisualizationPanel.tsx 中添加 `deviceColors` 颜色映射对象，统一管理 SVG 颜色

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Apple Design System
        'apple-blue': '#0071e3',
        'apple-blue-hover': '#0077ED',
        'apple-link': '#0066cc',
        'apple-link-dark': '#2997ff',

        // Morandi Palette
        'morandi-dark': '#1A1918',
        'morandi-light': '#F2EFE9',

        // Apple Dark Surfaces
        'dark-surface-1': '#272729',
        'dark-surface-2': '#262628',
        'dark-surface-3': '#28282a',
        'dark-surface-4': '#2a2a2d',
        'dark-surface-5': '#242426',

        // Apple Text Colors
        'apple-text-primary': '#ffffff',
        'apple-text-secondary': 'rgba(255, 255, 255, 0.8)',
        'apple-text-tertiary': 'rgba(255, 255, 255, 0.48)',

        // Button States
        'button-active': '#ededf2',
        'button-default-light': '#fafafc',

        // Log Level Colors
        'log-error': '#ff3b30',
        'log-warn': '#ff9500',
        'log-info': '#0071e3',
        'log-debug': '#8e8e93',
      },
      fontFamily: {
        'sf-pro': ['SF Pro Display', 'SF Pro Text', 'Helvetica Neue', 'Helvetica', 'Arial', 'sans-serif'],
        'mono': ['ui-monospace', 'SFMono-Regular', 'SF Mono', 'Menlo', 'Monaco', 'Consolas', 'monospace'],
      },
      boxShadow: {
        'apple-card': '3px 5px 30px rgba(0, 0, 0, 0.22)',
        'apple-card-hover': '3px 5px 40px rgba(0, 0, 0, 0.28)',
      },
      backdropBlur: {
        'glass': '30px',
      },
      borderRadius: {
        'apple-sm': '5px',
        'apple-md': '8px',
        'apple-lg': '12px',
        'apple-pill': '980px',
      },
    },
  },
  plugins: [],
}

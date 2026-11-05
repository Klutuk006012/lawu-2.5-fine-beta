import typography from '@tailwindcss/typography'

/**
 * Tailwind config untuk Lawu-2.5-Fine (Dashboard v3.6.0).
 * Palet warna: hijau emerald gunung Lawu (primary), merah Indonesia (accent),
 * amber cream (secondary), warm cream (background). Hindari blue/indigo/zinc cold.
 *
 * Format "R G B" triplet supaya alpha placeholder jalan:
 *   bg-primary/70  →  rgb(var(--primary) / 0.7)
 */
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: [
          'Inter',
          'ui-sans-serif',
          'system-ui',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'Roboto',
          'Helvetica Neue',
          'Arial',
          'sans-serif',
        ],
        mono: [
          'ui-monospace',
          'SFMono-Regular',
          'Menlo',
          'Monaco',
          'Consolas',
          'Liberation Mono',
          'Courier New',
          'monospace',
        ],
      },
      colors: {
        background: 'rgb(var(--background) / <alpha-value>)',
        foreground: 'rgb(var(--foreground) / <alpha-value>)',
        card: 'rgb(var(--card) / <alpha-value>)',
        'card-foreground': 'rgb(var(--card-foreground) / <alpha-value>)',
        primary: 'rgb(var(--primary) / <alpha-value>)',
        'primary-foreground': 'rgb(var(--primary-foreground) / <alpha-value>)',
        secondary: 'rgb(var(--secondary) / <alpha-value>)',
        'secondary-foreground': 'rgb(var(--secondary-foreground) / <alpha-value>)',
        muted: 'rgb(var(--muted) / <alpha-value>)',
        'muted-foreground': 'rgb(var(--muted-foreground) / <alpha-value>)',
        border: 'rgb(var(--border) / <alpha-value>)',
        accent: 'rgb(var(--accent) / <alpha-value>)',
        'accent-foreground': 'rgb(var(--accent-foreground) / <alpha-value>)',
        destructive: 'rgb(var(--destructive) / <alpha-value>)',
        'destructive-foreground': 'rgb(var(--destructive-foreground) / <alpha-value>)',
        lawuSky: 'rgb(var(--lawu-sky) / <alpha-value>)',
      },
      backgroundImage: {
        // 3 puncak gunung Lawu (Hargo Dlemo + Cakrahayu + 1 simetri visual) — repeat tileable.
        'mountain-pattern': "url(\"data:image/svg+xml;utf8,%3Csvg%20xmlns='http://www.w3.org/2000/svg'%20width='120'%20height='40'%20viewBox='0%200%20120%2040'%3E%3Cpath%20d='M0%2040%20L20%2015%20L35%2028%20L55%205%20L75%2028%20L90%2015%20L120%2040%20Z'%20fill='%230f5132'%20fill-opacity='0.5'/%3E%3C/svg%3E\")",
      },
      boxShadow: {
        'lawu-sm': '0 1px 2px rgba(15, 81, 50, 0.06), 0 1px 1px rgba(0, 0, 0, 0.04)',
        'lawu-md': '0 4px 12px -2px rgba(15, 81, 50, 0.08), 0 2px 4px -1px rgba(0, 0, 0, 0.04)',
      },
      keyframes: {
        'lawu-pulse': {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.4' },
        },
      },
      animation: {
        'lawu-pulse': 'lawu-pulse 1.6s ease-in-out infinite',
      },
    },
  },
  plugins: [typography],
}

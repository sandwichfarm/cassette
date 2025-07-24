/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{svelte,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'cassette-purple': '#9333ea',
        'cassette-blue': '#3b82f6', 
        'cassette-dark': '#1e1b4b',
        'cassette-orange': '#f97316',
        'cassette-yellow': '#fbbf24',
        'cassette-red': '#ef4444',
        'cassette-silver': '#94a3b8',
        'cassette-black': '#0f172a',
        'cassette-chrome': '#e2e8f0',
      },
      animation: {
        'fade-in': 'fadeIn 0.5s ease-in-out',
        'slide-up': 'slideUp 0.5s ease-out',
        'cassette-spin': 'cassetteSpinSlow 8s linear infinite',
        'tape-roll': 'tapeRoll 3s ease-in-out infinite alternate',
        'retro-glow': 'retroGlow 2s ease-in-out infinite alternate',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(20px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        cassetteSpinSlow: {
          '0%': { transform: 'rotate(0deg)' },
          '100%': { transform: 'rotate(360deg)' },
        },
        tapeRoll: {
          '0%': { transform: 'scaleX(1)' },
          '100%': { transform: 'scaleX(1.2)' },
        },
        retroGlow: {
          '0%': { boxShadow: '0 0 20px rgba(147, 51, 234, 0.3)' },
          '100%': { boxShadow: '0 0 40px rgba(147, 51, 234, 0.8)' },
        },
      },
    },
  },
  plugins: [],
}


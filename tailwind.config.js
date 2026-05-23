/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js}'],
  theme: {
    extend: {
      colors: {
        bg: {
          DEFAULT: '#0f1115',
          soft: '#161922',
          panel: '#1c2030',
        },
        accent: {
          DEFAULT: '#7c9cff',
          soft: '#3b4a7a',
        },
      },
    },
  },
  plugins: [],
};

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.rs",
    "./templates/**/*.{html,html.tera}"
  ],
  theme: {
    extend: {
      fontFamily: {
        mono: ['SpaceMono', 'monospace'],
        oswald: ["Oswald", "sans-serif"],
      },
    },
  },
  plugins: [],
}


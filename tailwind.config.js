/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    relative: true,
    files: ["*.html", "./app/src/**/*.rs"],
  },
  theme: {
    extend: {
      fontFamily: {
        'bold2': ['Compaq2x'],
        'bold1': ['Compaq1x'],
        'thin8': ['CompaqThin8'],
        'thin14': ['CompaqThin14'],
        'thin16': ['CompaqThin16']
      },
      boxShadow: {
        'box': '5px 5px 0px black'
      }
    },
  },
  plugins: [],
  safelist: [
    'hidden',
    'text-slate',
  ]
}
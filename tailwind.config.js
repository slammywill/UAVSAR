/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./src/**/*.{html,js,svelte,ts}",
        "./index.html",
        "./src/**/*.svelte",
        "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
        extend: {},
    },
    plugins: [],
    // Make sure Tailwind processes even in production
    safelist: [
        // Add any classes that might be dynamically generated and need to be included
    ],
    // Important: These settings can help with production build issues
    future: {
        purgeLayersByDefault: true,
        removeDeprecatedGapUtilities: true,
    },
    // This can help ensure all styles are properly processed
    mode: "jit",
};

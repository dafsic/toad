/** @type {import('tailwindcss').Config} */
export default {
    content: [
        './index.html',
        './src/**/*.{ts,tsx}',
    ],
    theme: {
        extend: {
            colors: {
                background: '#0d1117',
                foreground: '#e6edf3',
                card: '#161b22',
                border: '#2d3748',
                primary: '#3fb950',
                'primary-foreground': '#0d1117',
                secondary: '#21262d',
                muted: '#21262d',
                'muted-foreground': '#8b949e',
                ring: '#3fb950',
            },
            borderRadius: { lg: '0.5rem', md: '0.375rem', sm: '0.25rem' },
        },
    },
    plugins: [],
}

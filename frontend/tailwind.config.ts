/** @type {import('tailwindcss').Config} */
export default {
    content: [
        './index.html',
        './src/**/*.{ts,tsx}',
    ],
    theme: {
        extend: {
            colors: {
                background: '#0a0e14',
                foreground: '#e6edf3',
                card: '#111820',
                border: '#2a3a50',
                primary: '#3fb950',
                'primary-foreground': '#0a0e14',
                secondary: '#1d2535',
                muted: '#1d2535',
                'muted-foreground': '#8b949e',
                ring: '#f26822',
                xmr: '#f26822',
            },
            borderRadius: { lg: '0.5rem', md: '0.375rem', sm: '0.25rem' },
        },
    },
    plugins: [],
}

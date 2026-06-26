/** @type {import('tailwindcss').Config} */
export default {
    content: [
        './index.html',
        './src/**/*.{ts,tsx}',
    ],
    theme: {
        extend: {
            fontFamily: {
                display: ['"Inter Tight"', 'Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
                sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
            },
            colors: {
                // Brand — Monero orange (#FF6600)
                primary: '#FF6600',
                'primary-bright': '#ff7a1f',
                'primary-deep': '#cc4f00',
                'on-primary': '#ffffff',

                // Dark surfaces (canvas-dark mode)
                'canvas-dark': '#000000',
                'canvas-light': '#ffffff',
                'surface-deep': '#0a0a0a',
                'surface-elevated': '#16181a',
                'surface-soft': '#f4f4f4',

                // Text on dark
                'on-dark': '#ffffff',
                'on-dark-mute': 'rgba(255,255,255,0.72)',

                // Light-canvas text tokens (for completeness)
                ink: '#191c1f',
                body: '#1f2226',
                charcoal: '#3a3d40',
                mute: '#505a63',
                ash: '#5c5e60',
                stone: '#8d969e',
                faint: '#c9c9cd',

                // Hairlines / dividers
                'hairline-light': '#e2e2e7',
                'hairline-dark': 'rgba(255,255,255,0.12)',
                'hairline-strong': '#191c1f',
                'divider-soft': 'rgba(255,255,255,0.06)',

                // Semantic / accent palette (from DESIGN.md)
                'accent-teal': '#00a87e',
                'accent-light-blue': '#007bc2',
                'accent-light-green': '#428619',
                'accent-green-text': '#006400',
                'accent-yellow': '#b09000',
                'accent-warning': '#ec7e00',
                'accent-pink': '#e61e49',
                'accent-danger': '#e23b4a',
                'accent-deep-red': '#8b0000',

                // Legacy aliases kept for compatibility with existing markup
                background: '#000000',
                foreground: '#ffffff',
                card: '#16181a',
                border: 'rgba(255,255,255,0.12)',
                secondary: '#16181a',
                muted: '#0a0a0a',
                'muted-foreground': 'rgba(255,255,255,0.72)',
                ring: '#FF6600',
                xmr: '#FF6600',

                // Trading semantic shortcuts (mapped onto DESIGN palette)
                buy: '#428619',
                sell: '#FF6600',
                'buy-hover': '#3a7516',
                'sell-hover': '#cc4f00',
            },
            borderRadius: {
                none: '0px',
                sm: '8px',
                md: '12px',
                lg: '20px',
                xl: '28px',
                full: '9999px',
            },
            spacing: {
                xxs: '4px',
                xs: '6px',
                md: '14px',
                xxl: '32px',
                xxxl: '48px',
                block: '80px',
                section: '88px',
                band: '120px',
            },
        },
    },
    plugins: [],
}

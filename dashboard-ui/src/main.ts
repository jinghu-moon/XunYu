import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './styles/variable.css'
import App from './App.vue'

import PrimeVue from 'primevue/config'
import Aura from '@primeuix/themes/aura'
import { definePreset } from '@primeuix/themes'

// 定制极致黑白灰（Geist 风格）的 PrimeVue Aura 预设
const NoirExt = definePreset(Aura, {
  primitive: {
    // 覆盖默认圆角系统，映射为 variable.css 里的 Geist Radius
    borderRadius: {
      none: '0',
      xs: 'calc(var(--radius-xs) * 1)',
      sm: 'calc(var(--radius-sm) * 1)',
      md: 'calc(var(--radius-md) * 1)',
      lg: 'calc(var(--radius-lg) * 1)',
      xl: 'calc(var(--radius-xl) * 1)',
    },
    // 将自带字号映射至内部结构
    fontSize: {
      xs: 'var(--text-xs)',
      sm: 'var(--text-sm)',
      base: 'var(--text-base)',
      md: 'var(--text-md)',
      lg: 'var(--text-lg)',
      xl: 'var(--text-xl)',
    }
  },
  semantic: {
    // 焦点圆环
    focusRing: {
      width: '2px',
      style: 'solid',
      color: 'var(--text-primary)',
      offset: '2px',
      shadow: 'none'
    },
    primary: {
      50: '#f6f6f6',   // var(--gray-200) Light
      100: '#ededed',  // var(--gray-300) Light / var(--gray-1000) Dark
      200: '#e5e5e5',  // var(--gray-400) Light
      300: '#d4d4d4',  // var(--gray-500) Light
      400: '#a3a3a3',  // var(--gray-600) Light
      500: '#8f8f8f',  // var(--gray-700) Light
      600: '#6b6b6b',  // var(--gray-800) Light / var(--gray-700) Dark
      700: '#525252',  // var(--gray-900) Light / var(--gray-600) Dark
      800: '#3d3d3d',  // var(--gray-500) Dark
      900: '#242424',  // var(--gray-300) Dark
      950: '#171717'   // var(--ds-background-2)
    },
    colorScheme: {
      light: {
        primary: {
          color: 'var(--text-primary)',
          inverseColor: 'var(--ds-background-1)',
          hoverColor: 'var(--text-secondary)',
          activeColor: 'var(--text-tertiary)'
        },
        surface: {
          0: '#ffffff',
          50: '#f6f6f6',
          100: '#ededed',
          200: '#e5e5e5',
          300: '#d4d4d4',
          400: '#a3a3a3',
          500: '#8f8f8f',
          600: '#6b6b6b',
          700: '#525252',
          800: '#2e2e2e',
          900: '#171717',
          950: '#0f0f0f' 
        }
      },
      dark: {
        primary: {
          color: 'var(--text-primary)',
          inverseColor: 'var(--ds-background-1)',
          hoverColor: 'var(--text-secondary)',
          activeColor: 'var(--text-tertiary)'
        },
        surface: {
          0: '#ffffff',
          50: '#0a0a0a',
          100: '#171717',
          200: '#242424',
          300: '#2e2e2e',
          400: '#3d3d3d',
          500: '#525252',
          600: '#6b6b6b',
          700: '#8a8a8a',
          800: '#a1a1a1',
          900: '#ededed',
          950: '#ffffff'
        }
      }
    }
  },
  components: {
    button: {
      colorScheme: {
        light: {
          secondary: {
            background: 'var(--ds-background-1)',
            hoverBackground: 'var(--ds-background-2)',
            activeBackground: 'var(--ds-color-3)',
            color: 'var(--text-primary)',
            borderColor: 'var(--border)'
          }
        },
        dark: {
          secondary: {
            background: 'var(--ds-background-1)',
            hoverBackground: 'var(--ds-background-2)',
            activeBackground: 'var(--ds-color-3)',
            color: 'var(--text-primary)',
            borderColor: 'var(--border)'
          }
        }
      }
    }
  }
} as any) // Type override for extended tokens not strictly typed by PrimeUIX

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(PrimeVue, {
  theme: {
    preset: NoirExt,
    options: {
      darkModeSelector: 'html:not(.light)' // 适配现有的以 html.light 区分明暗的逻辑
    }
  }
})

app.mount('#app')

# KLAT GUI Theme Specification

**Version**: G1.0  
**Status**: Draft - Ready for Review  
**Last Updated**: 2026-07-08  

---

## 1. Overview

Dokumen ini mendefinisikan sistem tema untuk KLAT OS, termasuk warna, tipografi, spacing, dan animasi. Tema ini memastikan konsistensi visual di seluruh aplikasi native.

---

## 2. Design Principles

| Principle | Description |
|-----------|-------------|
| **Clarity** | Clean, readable interfaces |
| **Consistency** | Uniform across all apps |
| **Performance** | Hardware-accelerated where possible |
| **Accessibility** | High contrast, readable text |
| **Identity** | Distinctive KLAT aesthetic |

---

## 3. Color Palette

### 3.1 Base Colors

```css
:root {
    /* KLAT Primary Palette */
    --color-primary:        #2563EB;  /* KLAT Blue */
    --color-primary-hover:  #1D4ED8;
    --color-primary-active: #1E40AF;
    
    /* Surface Colors */
    --color-background:     #0F172A;  /* Dark slate */
    --color-surface:        #1E293B;  /* Elevated surface */
    --color-surface-hover:  #334155;
    --color-overlay:        #475569;  /* Modal overlays */
    
    /* Text Colors */
    --color-text-primary:   #F8FAFC;  /* Primary text */
    --color-text-secondary: #94A3B8;  /* Secondary text */
    --color-text-disabled:  #64748B;  /* Disabled text */
    
    /* Border Colors */
    --color-border:         #334155;
    --color-border-hover:    #475569;
    --color-border-focus:    #2563EB;
    
    /* Semantic Colors */
    --color-success:        #10B981;  /* Green */
    --color-warning:        #F59E0B;  /* Amber */
    --color-error:          #EF4444;  /* Red */
    --color-info:           #3B82F6;  /* Blue */
    
    /* Accent Colors */
    --color-accent:         #8B5CF6;  /* Purple accent */
    --color-accent-hover:   #7C3AED;
}
```

### 3.2 Light Theme (Future)

```css
:root {
    --color-background:     #F8FAFC;
    --color-surface:        #FFFFFF;
    --color-surface-hover:  #F1F5F9;
    --color-text-primary:   #0F172A;
    --color-text-secondary: #475569;
    --color-border:         #E2E8F0;
}
```

---

## 4. Typography

### 4.1 Font Stack

```css
/* Primary: System fonts for performance */
--font-family: 
    "Inter",
    "SF Pro Display",
    -apple-system,
    BlinkMacSystemFont,
    "Segoe UI",
    Roboto,
    "Helvetica Neue",
    Arial,
    sans-serif;

/* Monospace: For code/terminal */
--font-family-mono:
    "JetBrains Mono",
    "SF Mono",
    "Fira Code",
    "Consolas",
    monospace;
```

### 4.2 Font Sizes

```css
:root {
    /* Type Scale (1.25 ratio) */
    --font-size-xs:   11px;   /* 0.6875rem */
    --font-size-sm:   13px;   /* 0.8125rem */
    --font-size-md:   15px;   /* 0.9375rem - Base */
    --font-size-lg:   18px;   /* 1.125rem */
    --font-size-xl:   20px;   /* 1.25rem */
    --font-size-2xl:  24px;   /* 1.5rem */
    --font-size-3xl:  30px;   /* 1.875rem */
    --font-size-4xl:  36px;   /* 2.25rem */
}
```

### 4.3 Font Weights

```css
:root {
    --font-weight-normal:   400;
    --font-weight-medium:   500;
    --font-weight-semibold: 600;
    --font-weight-bold:     700;
}
```

### 4.4 Line Heights

```css
:root {
    --line-height-tight:   1.25;  /* Headings */
    --line-height-normal:   1.5;   /* Body text */
    --line-height-relaxed: 1.75;  /* Long text */
}
```

---

## 5. Spacing System

### 5.1 Base Unit

```css
:root {
    --spacing-unit: 4px;  /* Base unit */
}
```

### 5.2 Spacing Scale

```css
:root {
    --spacing-0:   0px;
    --spacing-px:  1px;
    --spacing-0_5: 2px;   /* 0.5 unit */
    --spacing-1:   4px;   /* 1 unit */
    --spacing-2:   8px;   /* 2 units */
    --spacing-3:   12px;  /* 3 units */
    --spacing-4:   16px;  /* 4 units */
    --spacing-5:   20px;  /* 5 units */
    --spacing-6:   24px;  /* 6 units */
    --spacing-8:   32px;  /* 8 units */
    --spacing-10:  40px;  /* 10 units */
    --spacing-12:  48px;  /* 12 units */
    --spacing-16:  64px;  /* 16 units */
    --spacing-20:  80px;  /* 20 units */
    --spacing-24:  96px;  /* 24 units */
}
```

---

## 6. Border Radius

### 6.1 Radius Scale

```css
:root {
    /* Sharp corners (inputs, buttons) */
    --radius-none:   0px;
    
    /* Subtle rounding */
    --radius-sm:     4px;
    
    /* Default rounding */
    --radius-md:     8px;
    
    /* Large rounding (cards, panels) */
    --radius-lg:     12px;
    
    /* Extra large (modals) */
    --radius-xl:     16px;
    
    /* Full circle */
    --radius-full:   9999px;
}
```

### 6.2 Component Usage

| Component | Radius |
|-----------|--------|
| Buttons | `--radius-md` |
| Inputs | `--radius-sm` |
| Cards | `--radius-lg` |
| Modals | `--radius-xl` |
| Avatars | `--radius-full` |
| Badges | `--radius-full` |
| Tooltips | `--radius-sm` |

---

## 7. Shadows / Elevation

### 7.1 Elevation Levels

```css
:root {
    /* No shadow */
    --shadow-none:   none;
    
    /* Subtle - buttons, inputs */
    --shadow-sm:  0 1px 2px rgba(0, 0, 0, 0.3);
    
    /* Card elevation */
    --shadow-md:  0 4px 6px rgba(0, 0, 0, 0.3),
                 0 2px 4px rgba(0, 0, 0, 0.2);
    
    /* Modal elevation */
    --shadow-lg:  0 10px 15px rgba(0, 0, 0, 0.3),
                 0 4px 6px rgba(0, 0, 0, 0.2);
    
    /* Popup elevation */
    --shadow-xl:  0 20px 25px rgba(0, 0, 0, 0.4),
                 0 8px 10px rgba(0, 0, 0, 0.2);
    
    /* Glow effect */
    --shadow-glow: 0 0 20px rgba(37, 99, 235, 0.4);
}
```

---

## 8. Animation

### 8.1 Duration Scale

```css
:root {
    /* Instant - for feedback */
    --duration-instant: 0ms;
    
    /* Fast - hover states, small changes */
    --duration-fast:    100ms;
    
    /* Normal - standard transitions */
    --duration-normal:  200ms;
    
    /* Slow - modals, large changes */
    --duration-slow:   300ms;
    
    /* Deliberate - emphasis */
    --duration-deliberate: 400ms;
}
```

### 8.2 Easing Functions

```css
:root {
    /* Standard - most transitions */
    --ease-default: cubic-bezier(0.4, 0, 0.2, 1);
    
    /* Enter - elements appearing */
    --ease-in:      cubic-bezier(0.4, 0, 1, 1);
    
    /* Exit - elements leaving */
    --ease-out:     cubic-bezier(0, 0, 0.2, 1);
    
    /* Bounce - playful elements */
    --ease-bounce:  cubic-bezier(0.68, -0.55, 0.265, 1.55);
    
    /* Linear - for progress indicators */
    --ease-linear:  linear;
}
```

### 8.3 Common Animations

| Animation | Duration | Easing | Description |
|-----------|----------|---------|-------------|
| Fade In | `--duration-fast` | `--ease-out` | Opacity 0→1 |
| Fade Out | `--duration-fast` | `--ease-in` | Opacity 1→0 |
| Slide Up | `--duration-normal` | `--ease-out` | Translate Y +20px→0 |
| Scale | `--duration-fast` | `--ease-bounce` | Scale 0.95→1 |
| Color | `--duration-fast` | `--ease-default` | Color transition |

---

## 9. Component Styles

### 9.1 Button

```css
.button {
    /* Sizing */
    padding: var(--spacing-2) var(--spacing-4);
    min-height: 36px;
    min-width: 80px;
    
    /* Border */
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    
    /* Text */
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-medium);
    
    /* Interactive */
    cursor: pointer;
    transition: all var(--duration-fast) var(--ease-default);
}

.button:hover {
    background-color: var(--color-surface-hover);
}

.button:active {
    transform: scale(0.98);
}

.button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}
```

### 9.2 Input

```css
.input {
    /* Sizing */
    padding: var(--spacing-2) var(--spacing-3);
    min-height: 36px;
    
    /* Border */
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    
    /* Text */
    font-size: var(--font-size-md);
    
    /* Interactive */
    transition: border-color var(--duration-fast) var(--ease-default);
}

.input:focus {
    outline: none;
    border-color: var(--color-border-focus);
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.2);
}
```

### 9.3 Card

```css
.card {
    /* Surface */
    background: var(--color-surface);
    border-radius: var(--radius-lg);
    
    /* Spacing */
    padding: var(--spacing-4);
    
    /* Shadow */
    box-shadow: var(--shadow-md);
}
```

### 9.4 Modal

```css
.modal {
    /* Backdrop */
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    
    /* Content */
    background: var(--color-surface);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-xl);
    
    /* Animation */
    animation: modal-in var(--duration-normal) var(--ease-out);
}

@keyframes modal-in {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}
```

---

## 10. Icon System

### 10.1 Icon Sizes

| Size | Pixels | Usage |
|------|--------|-------|
| `--icon-xs` | 16px | Inline text icons |
| `--icon-sm` | 20px | Toolbar icons |
| `--icon-md` | 24px | Default icons |
| `--icon-lg` | 32px | Large icons |
| `--icon-xl` | 48px | Feature icons |

### 10.2 Icon Grid

```css
:root {
    /* Material Design 24px grid */
    --icon-grid: 24px;
    
    /* Icon stroke width */
    --icon-stroke-width: 1.5px;
}
```

---

## 11. Accessibility

### 11.1 Focus States

```css
:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
}
```

### 11.2 Contrast Requirements

| Text Type | Minimum Contrast |
|-----------|------------------|
| Large text (≥18px bold, ≥14px regular) | 3:1 |
| Normal text | 4.5:1 |
| UI components and graphical objects | 3:1 |

### 11.3 Touch Targets

```css
:root {
    /* Minimum touch target size */
    --touch-target-min: 44px;  /* 48px in iOS guidelines */
}
```

---

## 12. Dark Theme Implementation

```css
@media (prefers-color-scheme: dark) {
    :root {
        --color-background:     #0F172A;
        --color-surface:        #1E293B;
        --color-surface-hover:  #334155;
        --color-text-primary:   #F8FAFC;
        --color-text-secondary: #94A3B8;
        --color-border:         #334155;
    }
}
```

---

## 13. Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-07-08 | G1.0 | Initial theme specification |

---

**Status**: Draft for Review  
**Next Step**: Review colors and typography scales  
**Maintainer**: KLAT OS Team

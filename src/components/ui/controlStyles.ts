export type ButtonLook = 'default' | 'primary' | 'dark' | 'danger' | 'quiet'

const LOOKS: Record<ButtonLook, string> = {
  default: 'border-ink-strong text-ink-strong hover:enabled:bg-hover disabled:border-separator disabled:text-ink-disabled',
  primary: 'border-primary bg-primary text-white hover:enabled:border-primary-hover hover:enabled:bg-primary-hover disabled:border-separator disabled:bg-separator disabled:text-ink-disabled',
  dark: 'border-black bg-black text-white hover:enabled:border-ink-strong hover:enabled:bg-ink-strong',
  danger: 'border-negative bg-negative text-white hover:enabled:border-negative-hover hover:enabled:bg-negative-hover',
  quiet: 'border-transparent text-ink-strong hover:enabled:bg-hover disabled:text-ink-disabled',
}

export const buttonClass = (look: ButtonLook = 'default') =>
  `inline-flex min-h-8 cursor-pointer items-center justify-center gap-1.5 rounded-pill border-2 px-3.5 pt-[3px] pb-[5px] leading-[18px] font-bold whitespace-nowrap no-underline disabled:cursor-default ${LOOKS[look]}`

const ICON_LOOKS = {
  plain: 'text-ink hover:enabled:bg-hover hover:enabled:text-ink-strong active:enabled:bg-pressed',
  selected: 'bg-accent text-white',
  toggled: 'bg-toggle text-white',
}

export const iconButtonClass = (look: 'selected' | 'toggled' | null = null) =>
  `group/tip relative inline-flex size-8 flex-none cursor-pointer items-center justify-center rounded border border-transparent disabled:cursor-default disabled:text-ink-disabled ${ICON_LOOKS[look ?? 'plain']}`

export const FIELD = 'rounded border border-field-border bg-white text-field-ink focus-visible:outline-2 focus-visible:-outline-offset-1 focus-visible:outline-selection'

export const REGULAR = 'min-h-8 px-2.5 py-1'
export const COMPACT = 'min-h-7 px-2 py-0.5'
export const CHEVRON = 'select-chevron pr-[30px]'

export const RESIZE_HANDLE = 'absolute -right-[5px] -bottom-[5px] size-[9px] cursor-nwse-resize border border-selection bg-surface'

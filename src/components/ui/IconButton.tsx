import type { MouseEvent } from 'react'
import { Icon, type IconName } from '@/components/ui/Icon.tsx'
import { Tooltip, type TooltipSide } from '@/components/ui/Tooltip.tsx'
import { iconButtonClass } from '@/components/ui/controlStyles.ts'

export interface IconButtonProps {
  icon: IconName
  label: string
  tooltip?: TooltipSide | null
  look?: 'selected' | 'toggled' | null
  pressed?: boolean
  flyout?: boolean
  disabled?: boolean
  onClick: (event: MouseEvent<HTMLButtonElement>) => void
}

export function IconButton({ icon, label, tooltip = 'below', look = null, pressed, flyout = false, disabled = false, onClick }: IconButtonProps) {
  return (
    <button
      type="button"
      className={iconButtonClass(look)}
      aria-label={label}
      aria-pressed={pressed}
      aria-haspopup={flyout ? 'menu' : undefined}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} />
      {flyout && <span className="absolute right-0.5 bottom-0.5 border-r-4 border-b-4 border-r-transparent border-b-current" aria-hidden="true" />}
      {tooltip && <Tooltip side={tooltip}>{label}</Tooltip>}
    </button>
  )
}

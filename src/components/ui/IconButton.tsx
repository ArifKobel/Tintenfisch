import type { MouseEvent } from 'react'
import { Icon, type IconName } from '@/components/ui/Icon.tsx'

export type TooltipSide = 'right' | 'left' | 'below'

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
  const classes = ['icon-button', look && `icon-button-${look}`, flyout && 'icon-button-flyout']
  return (
    <button type="button" className={classes.filter(Boolean).join(' ')} aria-label={label} aria-pressed={pressed} aria-haspopup={flyout ? 'menu' : undefined} disabled={disabled} onClick={onClick}>
      <Icon name={icon} />
      {tooltip && (
        <span className={`tooltip tooltip-${tooltip}`} aria-hidden="true">
          {label}
        </span>
      )}
    </button>
  )
}

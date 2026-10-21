import { useEffect, useRef, useState, type ComponentProps, type ReactNode } from 'react'
import { IconButton, type IconButtonProps } from '@/components/ui/IconButton.tsx'

export interface MenuButtonProps extends Omit<IconButtonProps, 'onClick' | 'flyout'> {
  opens: 'right' | 'below'
  children: (close: () => void) => ReactNode
}

const PLACES = {
  right: '-top-1 left-[calc(100%+12px)] max-md:top-auto max-md:bottom-[calc(100%+12px)] max-md:left-0',
  below: 'top-[calc(100%+6px)] left-0 max-md:right-0 max-md:left-auto',
}

export function MenuButton({ opens, children, ...button }: MenuButtonProps) {
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const dismiss = (event: Event) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !root.current?.contains(event.target as Node)) setOpen(false)
    }
    window.addEventListener('pointerdown', dismiss)
    window.addEventListener('keydown', dismiss)
    return () => {
      window.removeEventListener('pointerdown', dismiss)
      window.removeEventListener('keydown', dismiss)
    }
  }, [open])

  return (
    <div ref={root} className="relative">
      <IconButton {...button} flyout={opens === 'right'} pressed={open} tooltip={open ? null : button.tooltip} onClick={() => setOpen(!open)} />
      {open && (
        <div className={`absolute z-30 flex min-w-[180px] flex-col rounded border border-menu-border bg-surface p-1 shadow-menu ${PLACES[opens]}`} role="menu">
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  )
}

export function MenuItem({ chosen = false, className = '', ...props }: ComponentProps<'button'> & { chosen?: boolean }) {
  return (
    <button
      type="button"
      role="menuitem"
      className={`flex min-h-8 cursor-pointer items-center gap-2.5 rounded pr-3 pl-2 text-left whitespace-nowrap text-ink-strong hover:enabled:bg-hover focus-visible:bg-hover focus-visible:outline-none disabled:cursor-default disabled:text-ink-disabled ${chosen ? 'shadow-[inset_0_0_0_2px_var(--color-selection)]' : ''} ${className}`}
      {...props}
    />
  )
}

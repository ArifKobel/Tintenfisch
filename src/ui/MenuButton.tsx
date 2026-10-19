import { useEffect, useRef, useState, type ReactNode } from 'react'
import { IconButton, type IconButtonProps } from '@ui/IconButton.tsx'

export interface MenuButtonProps extends Omit<IconButtonProps, 'onClick' | 'flyout'> {
  opens: 'right' | 'below'
  children: (close: () => void) => ReactNode
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
    <div ref={root} className="menu-anchor">
      <IconButton {...button} flyout={opens === 'right'} pressed={open} tooltip={open ? null : button.tooltip} onClick={() => setOpen(!open)} />
      {open && (
        <div className={`menu menu-${opens}`} role="menu">
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  )
}

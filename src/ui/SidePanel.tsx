import type { KeyboardEvent, ReactNode } from 'react'
import { IconButton } from '@ui/IconButton.tsx'

export interface SidePanelProps {
  title: string
  side: 'left' | 'right'
  onClose?: () => void
  actions?: ReactNode
  onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void
  children: ReactNode
}

export function SidePanel({ title, side, onClose, actions, onKeyDown, children }: SidePanelProps) {
  const close = onClose && <IconButton icon="close" label={`Close ${title.toLowerCase()}`} tooltip={side === 'right' ? 'below' : 'left'} onClick={onClose} />
  return (
    <aside className={`panel panel-${side}`} aria-label={title} onKeyDown={onKeyDown}>
      <header className="panel-header">
        {side === 'right' && close}
        <h2 className="panel-title">{title}</h2>
        {actions}
        {side === 'left' && close}
      </header>
      <div className="panel-body">{children}</div>
    </aside>
  )
}

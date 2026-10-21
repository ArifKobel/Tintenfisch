import type { KeyboardEvent, ReactNode } from 'react'
import { IconButton } from '@/components/ui/IconButton.tsx'

export interface SidePanelProps {
  title: string
  side: 'left' | 'right'
  onClose?: () => void
  actions?: ReactNode
  onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void
  children: ReactNode
}

const SIDES = {
  left: 'w-[275px] border-r max-md:w-auto max-md:flex-1 max-md:border-r-0',
  right: 'w-[400px] border-l max-md:fixed max-md:inset-[56px_0_0] max-md:z-25 max-md:w-auto max-md:border-l-0',
}

export function SidePanel({ title, side, onClose, actions, onKeyDown, children }: SidePanelProps) {
  const close = onClose && <IconButton icon="close" label={`Close ${title.toLowerCase()}`} tooltip={side === 'right' ? 'below' : 'left'} onClick={onClose} />
  return (
    <aside className={`flex min-h-0 flex-none flex-col border-divider bg-surface ${SIDES[side]}`} aria-label={title} onKeyDown={onKeyDown}>
      <header className={`flex flex-none items-center gap-2 pt-4 pr-4 pb-2 ${side === 'left' ? 'pl-6 max-md:hidden' : 'pl-4'}`}>
        {side === 'right' && close}
        <h2 className="flex-1 truncate text-lg leading-6 font-bold text-ink-strong">{title}</h2>
        {actions}
        {side === 'left' && close}
      </header>
      <div className={`flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-2 pb-6 ${side === 'left' ? 'max-md:px-5 max-md:pt-3' : ''}`}>{children}</div>
    </aside>
  )
}

export function PanelSection({ children }: { children: ReactNode }) {
  return <section className="flex flex-col gap-1.5 [&+&]:border-t [&+&]:border-separator [&+&]:pt-4">{children}</section>
}

export function PanelActions({ children }: { children: ReactNode }) {
  return <div className="flex gap-1">{children}</div>
}

export function EmptyState({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mt-12 flex flex-col items-center gap-3 text-center text-[15px] text-ink-body">
      <h3 className="text-lg font-bold text-ink-strong">{title}</h3>
      {children}
    </div>
  )
}

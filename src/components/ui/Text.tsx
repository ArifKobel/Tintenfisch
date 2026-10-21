import type { ReactNode } from 'react'

export function Notice({ role = 'status', className = '', children }: { role?: 'status' | 'alert'; className?: string; children: ReactNode }) {
  return (
    <p className={`text-[13px] text-negative ${className}`} role={role}>
      {children}
    </p>
  )
}

export function Hint({ className = '', children }: { className?: string; children: ReactNode }) {
  return <p className={`text-[13px] text-ink-muted ${className}`}>{children}</p>
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return <h3 className="text-[11px] leading-[14px] tracking-[0.06em] text-ink uppercase">{children}</h3>
}

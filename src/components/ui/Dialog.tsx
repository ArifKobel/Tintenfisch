import type { FormEvent, ReactNode } from 'react'

interface DialogProps {
  title: ReactNode
  titleId?: string
  label?: string
  role?: 'dialog' | 'alertdialog'
  keepsEdit?: boolean
  className?: string
  onSubmit?: (event: FormEvent<HTMLFormElement>) => void
  children: ReactNode
}

const PANEL = 'flex max-h-[calc(100vh-64px)] max-w-[min(560px,calc(100vw-32px))] min-w-[360px] flex-col gap-4 overflow-y-auto rounded-dialog bg-surface px-10 py-8 text-ink shadow-dialog max-md:w-[calc(100vw-32px)] max-md:min-w-0 max-md:px-5 max-md:py-6'

export function DialogPanel({ title, titleId, label, role, keepsEdit, className = '', onSubmit, children }: DialogProps) {
  const heading = (
    <h2 id={titleId} className="border-b-2 border-separator pb-3.5 text-lg leading-6 font-bold text-ink-strong">
      {title}
    </h2>
  )
  const modal = {
    ...(role ? { role, 'aria-modal': true, 'aria-label': label, 'aria-labelledby': label ? undefined : titleId } : {}),
    'data-keeps-edit': keepsEdit || undefined,
  }
  if (onSubmit) {
    return (
      <form className={`${PANEL} ${className}`} onSubmit={onSubmit} {...modal}>
        {heading}
        {children}
      </form>
    )
  }
  return (
    <div className={`${PANEL} ${className}`} {...modal}>
      {heading}
      {children}
    </div>
  )
}

export function Dialog(props: DialogProps) {
  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/40">
      <DialogPanel role="dialog" {...props} />
    </div>
  )
}

export function DialogActions({ children }: { children: ReactNode }) {
  return <div className="mt-2 flex items-center justify-end gap-3">{children}</div>
}

export function DialogField({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-ink">
      {label}
      {children}
    </label>
  )
}

export function DialogChoice({ children }: { children: ReactNode }) {
  return <label className="flex items-center gap-2">{children}</label>
}

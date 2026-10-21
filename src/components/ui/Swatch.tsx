import type { ComponentProps } from 'react'

export function Swatch({ color, chosen, className = '', ...props }: ComponentProps<'button'> & { color: string; chosen: boolean }) {
  return (
    <button
      type="button"
      className={`size-5 cursor-pointer rounded-full border-2 border-surface p-0 ${chosen ? 'outline-2 outline-ink-strong' : 'outline-1 outline-black/20'} ${className}`}
      style={{ background: color }}
      {...props}
    />
  )
}

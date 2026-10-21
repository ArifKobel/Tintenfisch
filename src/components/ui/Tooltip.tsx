import type { ReactNode } from 'react'

export type TooltipSide = 'right' | 'left' | 'below'

const SIDES: Record<TooltipSide, string> = {
  right: 'top-1/2 left-[calc(100%+8px)] -translate-y-1/2',
  left: 'top-1/2 right-[calc(100%+8px)] -translate-y-1/2',
  below: 'top-[calc(100%+6px)] left-1/2 -translate-x-1/2',
}

export function Tooltip({ side, children }: { side: TooltipSide; children: ReactNode }) {
  return (
    <span
      className={`pointer-events-none absolute z-40 rounded bg-tooltip px-2 pt-0.5 pb-1 text-xs leading-[18px] font-normal whitespace-nowrap text-white opacity-0 transition-opacity delay-400 duration-120 group-hover/tip:opacity-100 group-focus-visible/tip:opacity-100 [@media(hover:none)]:hidden ${SIDES[side]}`}
      aria-hidden="true"
    >
      {children}
    </span>
  )
}

import type { ComponentProps } from 'react'
import { CHEVRON, COMPACT, FIELD, REGULAR } from '@/components/ui/controlStyles.ts'

interface Sized {
  compact?: boolean
}

export function Input({ compact = false, className = '', ...props }: ComponentProps<'input'> & Sized) {
  return <input className={`${FIELD} ${compact ? COMPACT : REGULAR} ${className}`} {...props} />
}

export function Select({ compact = false, className = '', ...props }: ComponentProps<'select'> & Sized) {
  return <select className={`${FIELD} ${CHEVRON} ${compact ? COMPACT : REGULAR} ${className}`} {...props} />
}

export function TextArea({ className = '', ...props }: ComponentProps<'textarea'>) {
  return <textarea className={`${FIELD} ${REGULAR} ${className}`} {...props} />
}

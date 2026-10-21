import type { ComponentProps } from 'react'
import { buttonClass, type ButtonLook } from '@/components/ui/controlStyles.ts'

export function Button({ look = 'default', className = '', type = 'button', ...props }: ComponentProps<'button'> & { look?: ButtonLook }) {
  return <button type={type} className={`${buttonClass(look)} ${className}`} {...props} />
}

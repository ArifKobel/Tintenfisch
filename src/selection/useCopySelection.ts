import { useEffect } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

export function useCopySelection() {
  const { selection } = useApplication()
  useEffect(() => {
    const copy = (event: ClipboardEvent) => {
      if (!selection.text || isEditable(event.target)) return
      event.clipboardData?.setData('text/plain', selection.text)
      event.preventDefault()
    }
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !isEditable(event.target)) selection.clear()
    }
    document.addEventListener('copy', copy)
    document.addEventListener('keydown', escape)
    return () => {
      document.removeEventListener('copy', copy)
      document.removeEventListener('keydown', escape)
    }
  }, [selection])
}

function isEditable(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement
}

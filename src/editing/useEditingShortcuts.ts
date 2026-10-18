import { useEffect } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

const NOT_TYPED = ['checkbox', 'radio', 'button', 'submit', 'file', 'range', 'color']

function typesText(target: EventTarget | null): boolean {
  if (target instanceof HTMLTextAreaElement) return !target.classList.contains('edit-input')
  return target instanceof HTMLInputElement && !NOT_TYPED.includes(target.type)
}

export function useEditingShortcuts() {
  const { editing } = useApplication()
  useEffect(() => {
    const revert = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'z' || typesText(event.target)) return
      event.preventDefault()
      void (event.shiftKey ? editing.redo() : editing.undo())
    }
    document.addEventListener('keydown', revert)
    return () => document.removeEventListener('keydown', revert)
  }, [editing])
}

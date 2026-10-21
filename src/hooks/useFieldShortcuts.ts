import { useEffect } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'

export function useFieldShortcuts() {
  const { forms, tools } = useApplication()
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement
      if (typing || tools.getSnapshot().tool !== 'form' || !forms.getSnapshot().selected) return
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault()
        void forms.deleteSelected()
      } else if (event.key === 'Escape') {
        forms.select(null)
      }
    }
    document.addEventListener('keydown', press)
    return () => document.removeEventListener('keydown', press)
  }, [forms, tools])
}

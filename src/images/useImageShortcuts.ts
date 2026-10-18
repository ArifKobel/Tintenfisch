import { useEffect } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

export function useImageShortcuts() {
  const { images } = useApplication()
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement
      if (typing || !images.getSnapshot().selected) return
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault()
        void images.removeSelected()
      } else if (event.key === 'Escape') {
        images.select(null)
      }
    }
    document.addEventListener('keydown', press)
    return () => document.removeEventListener('keydown', press)
  }, [images])
}

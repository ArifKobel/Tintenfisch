import { useEffect } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'

export function useLinkShortcuts() {
  const { links, tools } = useApplication()
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement
      const { selected, draft } = links.getSnapshot()
      if (typing || draft || tools.getSnapshot().tool !== 'link' || !selected) return
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault()
        void links.removeSelected()
      } else if (event.key === 'Escape') {
        links.select(null)
      }
    }
    document.addEventListener('keydown', press)
    return () => document.removeEventListener('keydown', press)
  }, [links, tools])
}

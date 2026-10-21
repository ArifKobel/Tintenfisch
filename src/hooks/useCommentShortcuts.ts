import { useEffect } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'

export function useCommentShortcuts() {
  const { comments } = useApplication()
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement
      if (typing || !comments.getSnapshot().selected) return
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault()
        void comments.removeSelected()
      } else if (event.key === 'Escape') {
        comments.select(null)
      }
    }
    document.addEventListener('keydown', press)
    return () => document.removeEventListener('keydown', press)
  }, [comments])
}

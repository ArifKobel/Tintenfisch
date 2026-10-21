import { useEffect, useRef } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { EditSession } from '@/lib/EditSession.ts'
import type { Highlight } from '@/types/protocol.ts'

const KEY_TARGETS: Record<string, (session: EditSession) => number> = {
  ArrowLeft: (session) => session.caret - 1,
  ArrowRight: (session) => session.caret + 1,
  Home: () => 0,
  End: (session) => session.clusters.length,
}

const COMMAND_KEYS: Record<string, (session: EditSession) => EditSession> = {
  a: (session) => session.selectAll(),
  b: (session) => session.formatted({ bold: !session.look.bold }),
  i: (session) => session.formatted({ italic: !session.look.italic }),
}

const INPUT_EDITS: Record<string, (session: EditSession, data: string) => EditSession> = {
  insertText: (session, data) => session.insert(data),
  insertLineBreak: (session) => session.insert('\n'),
  insertParagraph: (session) => session.insert('\n'),
  insertReplacementText: (session, data) => session.insert(data),
  deleteContentBackward: (session) => session.deleteBackward(),
  deleteContentForward: (session) => session.deleteForward(),
}

export function EditInput({ caret, cssScale }: { caret: Highlight | null; cssScale: number }) {
  const { editing } = useApplication()
  const input = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    const field = input.current
    if (!field) return
    const typed = (event: InputEvent) => {
      if (event.isComposing) return
      event.preventDefault()
      const change = INPUT_EDITS[event.inputType]
      if (change) editing.edit((session) => change(session, event.data ?? event.dataTransfer?.getData('text/plain') ?? ''))
    }
    const composed = (event: CompositionEvent) => {
      field.value = ''
      if (event.data) editing.edit((session) => session.insert(event.data))
    }
    const pressed = (event: KeyboardEvent) => {
      const target = KEY_TARGETS[event.key]
      const command = event.metaKey || event.ctrlKey ? COMMAND_KEYS[event.key.toLowerCase()] : undefined
      if (target) {
        event.preventDefault()
        editing.edit((session) => (event.shiftKey ? session.extendTo(target(session)) : session.moveTo(target(session))))
      } else if (command) {
        event.preventDefault()
        event.stopPropagation()
        editing.edit(command)
      } else if (event.key === 'Enter' && !event.isComposing) {
        event.preventDefault()
        editing.edit((session) => session.insert('\n'))
      } else if (event.key === 'Escape') {
        event.preventDefault()
        void editing.commit()
      }
    }
    const left = (event: FocusEvent) => {
      if (event.relatedTarget instanceof Element && event.relatedTarget.closest('.format-bar, .font-consent')) return
      void editing.commit()
    }
    field.addEventListener('beforeinput', typed)
    field.addEventListener('compositionend', composed)
    field.addEventListener('keydown', pressed)
    field.addEventListener('blur', left)
    return () => {
      field.removeEventListener('beforeinput', typed)
      field.removeEventListener('compositionend', composed)
      field.removeEventListener('keydown', pressed)
      field.removeEventListener('blur', left)
    }
  }, [editing])

  useEffect(() => input.current?.focus({ preventScroll: true }), [caret])

  return (
    <textarea
      ref={input}
      className="edit-input"
      aria-label="Edit paragraph"
      autoCapitalize="off"
      autoCorrect="off"
      spellCheck={false}
      style={{ left: (caret?.x ?? 0) * cssScale, top: (caret?.y ?? 0) * cssScale, height: (caret?.height ?? 0) * cssScale }}
    />
  )
}

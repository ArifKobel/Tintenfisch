import { create } from 'zustand'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { onDocumentChanged, onDocumentReplaced, useDocumentStore } from '@/stores/documentStore.ts'
import { useToastStore } from '@/stores/toastStore.ts'
import type { Box, FieldInput, FieldProperties, FormField, NewFieldKind } from '@/types/protocol.ts'

const DEFAULT_OPTIONS = ['Option 1', 'Option 2']

const NAMES: Record<NewFieldKind, string> = {
  text: 'Text',
  multiline: 'Notes',
  checkbox: 'Check',
  radio: 'Choice',
  combo: 'Dropdown',
  list: 'List',
}

type FieldChange =
  | { kind: 'setFieldProperties'; fieldId: string; properties: FieldProperties }
  | { kind: 'moveWidget'; pageId: string; widgetId: string; rect: Box }
  | { kind: 'deleteField'; fieldId: string }

interface FormsStore {
  fields: FormField[]
  selected: string | null
  reload(): Promise<void>
  fill(fieldId: string, input: FieldInput): Promise<void>
  select(fieldId: string | null): void
  addField(pageId: string, kind: NewFieldKind, rect: Box): Promise<void>
  setProperties(fieldId: string, properties: FieldProperties): Promise<void>
  moveWidget(pageId: string, widgetId: string, rect: Box): Promise<void>
  deleteSelected(): Promise<void>
  reset(): void
}

function unchanged(field: FormField, input: FieldInput): boolean {
  switch (input.kind) {
    case 'text':
      return field.kind === 'text' && field.text === input.text
    case 'checked':
      return (field.on !== null) === input.checked
    case 'radio':
      return field.widgets.find((widget) => widget.id === input.widget)?.onState === field.on
    case 'choices':
      return input.choices.join('\n') === field.chosen.join('\n')
  }
}

export const useFormsStore = create<FormsStore>()((set, get) => {
  let sequence = 0

  const attempt = async (work: () => Promise<void>) => {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) useToastStore.getState().show('forms', (error as Error).message)
    }
  }

  const change = (command: FieldChange) =>
    attempt(async () => {
      const changed = await engine.request(command)
      useDocumentStore.getState().touchPages(changed.dirtyPages)
      await get().reload()
    })

  return {
    fields: [],
    selected: null,

    async reload() {
      const loading = ++sequence
      await attempt(async () => {
        const fields = await engine.request({ kind: 'fields' }, { group: 'fields' })
        if (loading !== sequence) return
        const { selected } = get()
        set({ fields, selected: fields.some((field) => field.id === selected) ? selected : null })
        useToastStore.getState().clear('forms')
      })
    },

    async fill(fieldId, input) {
      const field = get().fields.find((candidate) => candidate.id === fieldId)
      if (!field || field.readOnly || unchanged(field, input)) return
      await attempt(async () => {
        const changed = await engine.request({ kind: 'setField', fieldId, input })
        useDocumentStore.getState().touchPages(changed.dirtyPages)
        await get().reload()
      })
    },

    select(fieldId) {
      if (fieldId !== get().selected) set({ selected: fieldId })
    },

    async addField(pageId, kind, rect) {
      const { fields, selected } = get()
      const current = fields.find((field) => field.id === selected)
      const name = kind === 'radio' && current?.kind === 'radio' ? current.name : NAMES[kind]
      const options = kind === 'combo' || kind === 'list' ? DEFAULT_OPTIONS : []
      await attempt(async () => {
        const added = await engine.request({ kind: 'addField', pageId, field: { kind, name, rect, options } })
        useDocumentStore.getState().touchPages(added.dirtyPages)
        await get().reload()
        set({ selected: added.fieldId })
      })
    },

    setProperties: (fieldId, properties) => change({ kind: 'setFieldProperties', fieldId, properties }),

    moveWidget: (pageId, widgetId, rect) => change({ kind: 'moveWidget', pageId, widgetId, rect }),

    async deleteSelected() {
      const fieldId = get().selected
      if (fieldId) await change({ kind: 'deleteField', fieldId })
    },

    reset() {
      sequence++
      set({ fields: [], selected: null })
      useToastStore.getState().clear('forms')
    },
  }
})

onDocumentReplaced((ready) => {
  useFormsStore.getState().reset()
  if (ready) void useFormsStore.getState().reload()
})

onDocumentChanged(() => void useFormsStore.getState().reload())

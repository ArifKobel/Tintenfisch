import { CancelledError, type EngineClient } from '@/services/EngineClient.ts'
import type { Box, FieldInput, FieldProperties, FormField, NewFieldKind } from '@/types/protocol.ts'
import type { PageRevisionStore } from '@/stores/PageRevisionStore.ts'

export interface FormsState {
  fields: FormField[]
  selected: string | null
  notice: string | null
}

const DEFAULT_OPTIONS = ['Option 1', 'Option 2']

const NAMES: Record<NewFieldKind, string> = {
  text: 'Text',
  multiline: 'Notes',
  checkbox: 'Check',
  radio: 'Choice',
  combo: 'Dropdown',
  list: 'List',
}

type Listener = () => void

export class FormsStore {
  readonly #client: EngineClient
  readonly #revisions: PageRevisionStore
  readonly #listeners = new Set<Listener>()
  #state: FormsState = { fields: [], selected: null, notice: null }
  #sequence = 0

  constructor(client: EngineClient, revisions: PageRevisionStore) {
    this.#client = client
    this.#revisions = revisions
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  readonly getSnapshot = (): FormsState => this.#state

  async reload() {
    const sequence = ++this.#sequence
    await this.#attempt(async () => {
      const fields = await this.#client.request({ kind: 'fields' }, { group: 'fields' })
      if (sequence !== this.#sequence) return
      const selected = fields.some((field) => field.id === this.#state.selected) ? this.#state.selected : null
      this.#update({ ...this.#state, fields, selected, notice: null })
    })
  }

  async fill(fieldId: string, input: FieldInput) {
    const field = this.#state.fields.find((candidate) => candidate.id === fieldId)
    if (!field || field.readOnly || FormsStore.#unchanged(field, input)) return
    await this.#attempt(async () => {
      const change = await this.#client.request({ kind: 'setField', fieldId, input })
      this.#revisions.touch(change.dirtyPages)
      await this.reload()
    })
  }

  select(fieldId: string | null) {
    if (fieldId !== this.#state.selected) this.#update({ ...this.#state, selected: fieldId })
  }

  async addField(pageId: string, kind: NewFieldKind, rect: Box) {
    const selected = this.#state.fields.find((field) => field.id === this.#state.selected)
    const name = kind === 'radio' && selected?.kind === 'radio' ? selected.name : NAMES[kind]
    const options = kind === 'combo' || kind === 'list' ? DEFAULT_OPTIONS : []
    await this.#attempt(async () => {
      const added = await this.#client.request({ kind: 'addField', pageId, field: { kind, name, rect, options } })
      this.#revisions.touch(added.dirtyPages)
      await this.reload()
      this.#update({ ...this.#state, selected: added.fieldId })
    })
  }

  async setProperties(fieldId: string, properties: FieldProperties) {
    await this.#change({ kind: 'setFieldProperties', fieldId, properties })
  }

  async moveWidget(pageId: string, widgetId: string, rect: Box) {
    await this.#change({ kind: 'moveWidget', pageId, widgetId, rect })
  }

  async deleteSelected() {
    const fieldId = this.#state.selected
    if (fieldId) await this.#change({ kind: 'deleteField', fieldId })
  }

  reset() {
    this.#sequence++
    this.#update({ fields: [], selected: null, notice: null })
  }

  async #change(
    command:
      | { kind: 'setFieldProperties'; fieldId: string; properties: FieldProperties }
      | { kind: 'moveWidget'; pageId: string; widgetId: string; rect: Box }
      | { kind: 'deleteField'; fieldId: string },
  ) {
    await this.#attempt(async () => {
      const change = await this.#client.request(command)
      this.#revisions.touch(change.dirtyPages)
      await this.reload()
    })
  }

  static #unchanged(field: FormField, input: FieldInput): boolean {
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

  async #attempt(work: () => Promise<void>) {
    try {
      await work()
    } catch (error) {
      if (!(error instanceof CancelledError)) this.#update({ ...this.#state, notice: (error as Error).message })
    }
  }

  #update(state: FormsState) {
    this.#state = state
    this.#listeners.forEach((listener) => listener())
  }
}

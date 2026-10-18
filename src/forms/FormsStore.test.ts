import { describe, expect, it } from 'vitest'
import { EngineClient } from '@engine/EngineClient.ts'
import type { FormField, Request } from '@engine/protocol.ts'
import { FormsStore } from '@forms/FormsStore.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'
import { PageRevisionStore } from '@viewer/PageRevisionStore.ts'

const widget = (id: string, onState: string | null = null) => ({ id, pageId: '4-0', rect: { x: 0, y: 0, width: 50, height: 20 }, onState })

const field = (overrides: Partial<FormField>): FormField => ({
  id: '11-0',
  name: 'person.first',
  partialName: 'first',
  kind: 'text',
  multiline: false,
  password: false,
  maxLength: null,
  editable: false,
  readOnly: false,
  required: false,
  text: 'Ada',
  on: null,
  chosen: [],
  options: [],
  widgets: [widget('11-0')],
  ...overrides,
})

const fields = [
  field({}),
  field({ id: '50-0', name: 'color', kind: 'radio', on: 'Red', widgets: [widget('51-0', 'Red'), widget('52-0', 'Blue')] }),
  field({ id: '80-0', name: 'id', readOnly: true }),
]

async function createStore() {
  const port = new FakeEnginePort()
  const revisions = new PageRevisionStore()
  const store = new FormsStore(new EngineClient(port), revisions)
  const loading = store.reload()
  await port.answerNext(fields)
  await loading
  return { port, revisions, store }
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('FormsStore', () => {
  it('fills a field, redraws its page and reloads the form', async () => {
    const { port, revisions, store } = await createStore()
    const filling = store.fill('11-0', { kind: 'text', text: 'Grace' })
    expect(last(port)).toMatchObject({ kind: 'setField', fieldId: '11-0', input: { kind: 'text', text: 'Grace' } })
    await port.answerNext({ dirtyPages: ['4-0'] })
    await port.answerNext([field({ text: 'Grace' })])
    await filling
    expect(revisions.revisionOf('4-0')).toBe(1)
    expect(store.getSnapshot().fields[0].text).toBe('Grace')
  })

  it('leaves the engine alone for unchanged values and read only fields', async () => {
    const { port, store } = await createStore()
    await store.fill('11-0', { kind: 'text', text: 'Ada' })
    await store.fill('50-0', { kind: 'radio', widget: '51-0' })
    await store.fill('80-0', { kind: 'text', text: 'B-1' })
    expect(port.requests).toHaveLength(1)
    const choosing = store.fill('50-0', { kind: 'radio', widget: '52-0' })
    expect(last(port)).toMatchObject({ kind: 'setField', input: { kind: 'radio', widget: '52-0' } })
    await port.answerNext({ dirtyPages: [] })
    await port.answerNext(fields)
    await choosing
  })
  it('names new fields by kind, lets a radio button join the selected group and selects the new field', async () => {
    const { port, store } = await createStore()
    const rect = { x: 10, y: 20, width: 150, height: 20 }
    const adding = store.addField('4-0', 'text', rect)
    expect(last(port)).toMatchObject({ kind: 'addField', pageId: '4-0', field: { kind: 'text', name: 'Text', rect, options: [] } })
    await port.answerNext({ fieldId: '90-0', dirtyPages: ['4-0'] })
    await port.answerNext([...fields, field({ id: '90-0', name: 'Text' })])
    await adding
    expect(store.getSnapshot().selected).toBe('90-0')
    store.select('50-0')
    const joining = store.addField('4-0', 'radio', { ...rect, width: 14, height: 14 })
    expect(last(port)).toMatchObject({ field: { kind: 'radio', name: 'color' } })
    await port.answerNext({ fieldId: '50-0', dirtyPages: ['4-0'] })
    await port.answerNext(fields)
    await joining
    const listing = store.addField('4-0', 'list', rect)
    expect(last(port)).toMatchObject({ field: { kind: 'list', name: 'List', options: ['Option 1', 'Option 2'] } })
    await port.answerNext({ fieldId: '91-0', dirtyPages: ['4-0'] })
    await port.answerNext(fields)
    await listing
  })

  it('changes, moves and deletes the selected field and forgets it once it is gone', async () => {
    const { port, store } = await createStore()
    const properties = { name: 'given', required: true, readOnly: false, multiline: false, options: [] }
    const changing = store.setProperties('11-0', properties)
    expect(last(port)).toMatchObject({ kind: 'setFieldProperties', fieldId: '11-0', properties })
    await port.answerNext({ dirtyPages: ['4-0'] })
    await port.answerNext(fields)
    await changing
    const moving = store.moveWidget('4-0', '11-0', { x: 1, y: 2, width: 3, height: 4 })
    expect(last(port)).toMatchObject({ kind: 'moveWidget', widgetId: '11-0', rect: { x: 1, y: 2, width: 3, height: 4 } })
    await port.answerNext({ dirtyPages: ['4-0'] })
    await port.answerNext(fields)
    await moving
    store.select('11-0')
    const deleting = store.deleteSelected()
    expect(last(port)).toMatchObject({ kind: 'deleteField', fieldId: '11-0' })
    await port.answerNext({ dirtyPages: ['4-0'] })
    await port.answerNext(fields.slice(1))
    await deleting
    expect(store.getSnapshot().selected).toBeNull()
  })
})

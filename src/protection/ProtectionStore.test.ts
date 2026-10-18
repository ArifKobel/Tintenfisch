import { describe, expect, it } from 'vitest'
import { EngineClient } from '@engine/EngineClient.ts'
import type { Permissions, Request } from '@engine/protocol.ts'
import { ProtectionStore } from '@protection/ProtectionStore.ts'
import { FakeEnginePort } from '@testing/FakeEnginePort.ts'

const everything: Permissions = { print: true, printFaithful: true, modify: true, copy: true, annotate: true, fillForms: true, assemble: true }
const locked = { encrypted: true, ownerRights: false, permissions: { ...everything, copy: false } }

function createStore() {
  const port = new FakeEnginePort()
  const store = new ProtectionStore(new EngineClient(port), (length) => new Uint8Array(length).fill(7))
  return { port, store }
}

const last = (port: FakeEnginePort): Request => port.requests[port.requests.length - 1]

describe('ProtectionStore', () => {
  it('asks for the permissions password before the protection can change', async () => {
    const { port, store } = createStore()
    const opening = store.openDialog()
    await port.answerNext(locked)
    await opening
    expect(store.getSnapshot()).toMatchObject({ dialogOpen: true, current: locked })
    const wrong = store.grantOwnerRights('guess')
    expect(last(port)).toMatchObject({ kind: 'grantOwnerRights', password: 'guess' })
    await port.answerNext(false)
    await port.answerNext(locked)
    await wrong
    expect(store.getSnapshot().notice).toBe('That is not the permissions password')
    const right = store.grantOwnerRights('boss')
    await port.answerNext(true)
    await port.answerNext({ ...locked, ownerRights: true })
    await right
    expect(store.getSnapshot()).toMatchObject({ notice: null, current: { ownerRights: true } })
  })

  it('protects with fresh random bytes and says the protection comes with the next save', async () => {
    const { port, store } = createStore()
    const protecting = store.protect('open', 'boss', { ...everything, modify: false })
    const request = last(port) as Extract<Request, { kind: 'protect' }>
    expect(request).toMatchObject({ kind: 'protect', userPassword: 'open', ownerPassword: 'boss', permissions: { modify: false } })
    expect(request.entropy).toHaveLength(100)
    await port.answerNext(undefined)
    await protecting
    expect(store.getSnapshot()).toMatchObject({ dialogOpen: false, pending: 'The protection is written when you save the document' })
    const removing = store.unprotect()
    expect(last(port).kind).toBe('unprotect')
    await port.answerNext(undefined)
    await removing
    expect(store.getSnapshot().pending).toBe('The document is saved without protection from now on')
  })
})

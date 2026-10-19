import { describe, expect, it } from 'vitest'
import { EngineHost, type EngineBackend } from '@engine/EngineHost.ts'

function backend(overrides: Partial<EngineBackend> = {}): EngineBackend {
  return {
    open: (bytes: Uint8Array) => ({ byteLength: bytes.length }),
    unlock: () => ({ status: 'locked', wrongPassword: true }),
    pages: () => [{ id: '3-0', width: 612, height: 792, rotate: 0 }],
    renderTile: () =>
      ({ width: 2, height: 1, takePixels: () => new Uint8Array(8), free: () => {} }) as unknown as ReturnType<EngineBackend['renderTile']>,
    save: () => new Uint8Array([37, 80, 68, 70]),
    hitText: () => ({ line: 0, offset: 2 }),
    selectText: () => ({ anchor: { line: 0, offset: 0 }, focus: { line: 0, offset: 2 }, rects: [], text: 'ab' }),
    find: () => [],
    paragraphText: () => ({ line: 0, clusters: ['a', 'b'], start: 0, styles: [] }),
    previewEdit: () => ({ caret: null, frame: { x: 1, y: 2, width: 50, height: 10, angle: 0 } }),
    previewNewText: () => ({ caret: null, frame: { x: 1, y: 2, width: 50, height: 10, angle: 0 }, selection: [] }),
    pageFonts: () => [{ name: 'F1', font: 'Helvetica' }],
    images: () => [],
    insertImage: () => ({ index: 0, dirtyPages: ['3-0'] }),
    moveImage: () => ({ dirtyPages: ['3-0'] }),
    replaceImage: () => ({ dirtyPages: ['3-0'] }),
    deleteImage: () => ({ dirtyPages: ['3-0'] }),
    outline: () => [],
    protection: () => ({ encrypted: false, ownerRights: true, permissions: {} }),
    grantOwnerRights: () => true,
    protect: () => {},
    unprotect: () => {},
    addBookmark: () => '40-0',
    renameBookmark: () => {},
    retargetBookmark: () => {},
    moveBookmark: () => {},
    deleteBookmark: () => {},
    links: () => [],
    addLink: () => ({ id: '41-0', dirtyPages: ['3-0'] }),
    updateLink: () => ({ dirtyPages: ['3-0'] }),
    commitEdit: () => ({ dirtyPages: ['3-0'] }),
    cancelEdit: () => {},
    undo: () => ({ dirtyPages: ['3-0'], uiState: '{}' }),
    redo: () => ({ dirtyPages: [] }),
    rotatePages: () => ({ pages: [], dirtyPages: [] }),
    deletePages: () => ({ pages: [], dirtyPages: [] }),
    movePages: () => ({ pages: [], dirtyPages: [] }),
    insertBlankPage: () => ({ pages: [], dirtyPages: [] }),
    insertDocument: () => ({ status: 'locked', wrongPassword: false }),
    extractPages: () => new Uint8Array([37, 80]),
    comments: () => [],
    addMarkup: () => ({ id: '20-0', dirtyPages: ['3-0'] }),
    deleteComment: () => ({ dirtyPages: ['3-0'] }),
    setCommentContents: () => ({ dirtyPages: [] }),
    applyRedactions: () => ({ dirtyPages: ['3-0'] }),
    addInkSignature: () => ({ fieldId: '70-0', dirtyPages: ['3-0'] }),
    addImageSignature: () => ({ fieldId: '71-0', dirtyPages: ['3-0'] }),
    signField: () => new Uint8Array([37, 80, 68, 70]),
    signatures: () => [],
    fields: () => [],
    setField: () => ({ dirtyPages: ['3-0'] }),
    addField: () => ({ fieldId: '90-0', dirtyPages: ['3-0'] }),
    setFieldProperties: () => ({ dirtyPages: ['3-0'] }),
    moveWidget: () => ({ dirtyPages: ['3-0'] }),
    deleteField: () => ({ dirtyPages: ['3-0'] }),
    revision: () => 4,
    ...overrides,
  }
}

describe('EngineHost', () => {
  it('answers a command with the engine result under the request id', () => {
    const { response, transfer } = new EngineHost(backend()).handle({ id: 7, priority: 'interactive', kind: 'open', bytes: new Uint8Array(3) })
    expect(response).toEqual({ id: 7, ok: true, result: { byteLength: 3 }, revision: 4 })
    expect(transfer).toEqual([])
  })

  it('returns tile pixels as a transferable buffer', () => {
    const { response, transfer } = new EngineHost(backend()).handle({
      id: 4,
      priority: 'visible',
      kind: 'renderTile',
      pageId: '3-0',
      scale: 2,
      x: 0,
      y: 0,
      width: 2,
      height: 1,
    })
    expect(response.ok && response.result).toMatchObject({ width: 2, height: 1 })
    expect(transfer).toHaveLength(1)
  })

  it('transfers saved bytes back instead of copying them', () => {
    const { transfer } = new EngineHost(backend()).handle({ id: 9, priority: 'interactive', kind: 'save', optimize: false })
    expect(transfer).toHaveLength(1)
  })

  it('passes an edit preview through to the engine', () => {
    const calls: unknown[][] = []
    const host = new EngineHost(
      backend({
        previewEdit: (...args: unknown[]) => {
          calls.push(args)
          return null
        },
      }),
    )
    const draft = { text: 'new', caret: 1, dx: 4, dy: 0, width: null, styles: [{ start: 0, end: 2, bold: true }], selection: [0, 2] as [number, number] }
    host.handle({ id: 3, priority: 'interactive', kind: 'previewEdit', pageId: '3-0', line: 2, draft })
    expect(calls).toEqual([['3-0', 2, draft]])
  })

  it('turns an engine failure into an error response instead of throwing', () => {
    const host = new EngineHost(
      backend({
        open: () => {
          throw new Error('not a PDF file')
        },
      }),
    )
    const { response } = host.handle({ id: 8, priority: 'interactive', kind: 'open', bytes: new Uint8Array(0) })
    expect(response).toEqual({ id: 8, ok: false, error: 'not a PDF file' })
  })
})

import type { Engine } from '@wasm/tintenfisch_wasm.js'
import type { Command, RenderedTile, Request, Response } from '@engine/protocol.ts'

export type EngineBackend = Pick<
  Engine,
  | 'open'
  | 'unlock'
  | 'pages'
  | 'renderTile'
  | 'save'
  | 'hitText'
  | 'selectText'
  | 'find'
  | 'paragraphText'
  | 'previewEdit'
  | 'previewNewText'
  | 'pageFonts'
  | 'registerFontReplacement'
  | 'images'
  | 'insertImage'
  | 'moveImage'
  | 'replaceImage'
  | 'deleteImage'
  | 'outline'
  | 'protection'
  | 'grantOwnerRights'
  | 'protect'
  | 'unprotect'
  | 'addBookmark'
  | 'renameBookmark'
  | 'retargetBookmark'
  | 'moveBookmark'
  | 'deleteBookmark'
  | 'links'
  | 'addLink'
  | 'updateLink'
  | 'commitEdit'
  | 'cancelEdit'
  | 'undo'
  | 'redo'
  | 'rotatePages'
  | 'deletePages'
  | 'movePages'
  | 'insertBlankPage'
  | 'insertDocument'
  | 'extractPages'
  | 'comments'
  | 'addMarkup'
  | 'deleteComment'
  | 'setCommentContents'
  | 'applyRedactions'
  | 'fields'
  | 'setField'
  | 'addField'
  | 'setFieldProperties'
  | 'moveWidget'
  | 'deleteField'
  | 'addInkSignature'
  | 'addImageSignature'
  | 'signField'
  | 'signatures'
  | 'revision'
>

export interface HostReply {
  response: Response
  transfer: Transferable[]
}

export class EngineHost {
  readonly #engine: EngineBackend

  constructor(engine: EngineBackend) {
    this.#engine = engine
  }

  handle(request: Request): HostReply {
    try {
      const result = this.#execute(request)
      return { response: { id: request.id, ok: true, result, revision: this.#engine.revision() }, transfer: EngineHost.#transferablesOf(result) }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      return { response: { id: request.id, ok: false, error: message }, transfer: [] }
    }
  }

  #execute(command: Command): unknown {
    switch (command.kind) {
      case 'open':
        return this.#engine.open(command.bytes)
      case 'unlock':
        return this.#engine.unlock(command.password)
      case 'pages':
        return this.#engine.pages()
      case 'renderTile':
        return this.#renderTile(command)
      case 'save':
        return this.#engine.save(command.optimize)
      case 'hitText':
        return this.#engine.hitText(command.pageId, command.x, command.y)
      case 'selectText': {
        const { anchor, focus } = command
        return this.#engine.selectText(command.pageId, new Uint32Array([anchor.line, anchor.offset]), new Uint32Array([focus.line, focus.offset]), command.unit)
      }
      case 'find':
        return this.#engine.find(command.query)
      case 'paragraphText':
        return this.#engine.paragraphText(command.pageId, command.line)
      case 'previewEdit':
        return this.#engine.previewEdit(command.pageId, command.line, command.draft)
      case 'previewNewText':
        return this.#engine.previewNewText(command.pageId, command.draft)
      case 'pageFonts':
        return this.#engine.pageFonts(command.pageId)
      case 'registerFontReplacement':
        return this.#engine.registerFontReplacement(command.baseFont, command.bytes)
      case 'images':
        return this.#engine.images(command.pageId)
      case 'insertImage':
        return this.#engine.insertImage(command.pageId, command.bytes, command.x, command.y)
      case 'moveImage':
        return this.#engine.moveImage(command.pageId, command.index, command.frame)
      case 'replaceImage':
        return this.#engine.replaceImage(command.pageId, command.index, command.bytes)
      case 'deleteImage':
        return this.#engine.deleteImage(command.pageId, command.index)
      case 'outline':
        return this.#engine.outline()
      case 'protection':
        return this.#engine.protection()
      case 'grantOwnerRights':
        return this.#engine.grantOwnerRights(command.password)
      case 'protect':
        return this.#engine.protect(command.userPassword, command.ownerPassword, command.permissions, command.entropy)
      case 'unprotect':
        return this.#engine.unprotect()
      case 'addBookmark':
        return this.#engine.addBookmark(command.parent ?? undefined, command.index, command.title, command.target)
      case 'renameBookmark':
        return this.#engine.renameBookmark(command.bookmarkId, command.title)
      case 'retargetBookmark':
        return this.#engine.retargetBookmark(command.bookmarkId, command.target)
      case 'moveBookmark':
        return this.#engine.moveBookmark(command.bookmarkId, command.parent ?? undefined, command.index)
      case 'deleteBookmark':
        return this.#engine.deleteBookmark(command.bookmarkId)
      case 'links':
        return this.#engine.links(command.pageId)
      case 'addLink':
        return this.#engine.addLink(command.pageId, command.rect, command.target)
      case 'updateLink':
        return this.#engine.updateLink(command.pageId, command.linkId, command.rect, command.target)
      case 'commitEdit':
        return this.#engine.commitEdit(command.uiState)
      case 'cancelEdit':
        return this.#engine.cancelEdit()
      case 'rotatePages':
        return this.#engine.rotatePages(command.pageIds, command.quarterTurns)
      case 'deletePages':
        return this.#engine.deletePages(command.pageIds)
      case 'movePages':
        return this.#engine.movePages(command.pageIds, command.before)
      case 'insertBlankPage':
        return this.#engine.insertBlankPage(command.before, command.width, command.height)
      case 'insertDocument':
        return this.#engine.insertDocument(command.bytes, command.password, command.before)
      case 'extractPages':
        return this.#engine.extractPages(command.pageIds)
      case 'comments':
        return this.#engine.comments()
      case 'addMarkup':
        return this.#engine.addMarkup(command.pageId, command.markup, command.style)
      case 'deleteComment':
        return this.#engine.deleteComment(command.pageId, command.commentId)
      case 'setCommentContents':
        return this.#engine.setCommentContents(command.commentId, command.contents, command.date)
      case 'applyRedactions':
        return this.#engine.applyRedactions()
      case 'fields':
        return this.#engine.fields()
      case 'setField':
        return this.#engine.setField(command.fieldId, command.input)
      case 'addField':
        return this.#engine.addField(command.pageId, command.field)
      case 'setFieldProperties':
        return this.#engine.setFieldProperties(command.fieldId, command.properties)
      case 'moveWidget': {
        const { x, y, width, height } = command.rect
        return this.#engine.moveWidget(command.pageId, command.widgetId, x, y, width, height)
      }
      case 'deleteField':
        return this.#engine.deleteField(command.fieldId)
      case 'addSignature': {
        const { pageId, rect, name, mark } = command
        if (mark.kind === 'ink') return this.#engine.addInkSignature(pageId, rect, name, { strokes: mark.strokes, penWidth: mark.penWidth, color: mark.color })
        return this.#engine.addImageSignature(pageId, rect, name, mark.width, mark.height, mark.rgba)
      }
      case 'signField':
        return this.#engine.signField(command.fieldId, command.certificate, command.password, command.reason, command.location, command.date)
      case 'signatures':
        return this.#engine.signatures()
      case 'undo':
        return this.#engine.undo()
      case 'redo':
        return this.#engine.redo()
    }
  }

  #renderTile(command: Extract<Command, { kind: 'renderTile' }>): RenderedTile {
    const tile = this.#engine.renderTile(command.pageId, command.scale, command.x, command.y, command.width, command.height)
    try {
      return { width: tile.width, height: tile.height, rgba: tile.takePixels() }
    } finally {
      tile.free()
    }
  }

  static #transferablesOf(result: unknown): Transferable[] {
    if (result instanceof Uint8Array) return [result.buffer as ArrayBuffer]
    if (typeof result === 'object' && result !== null && 'rgba' in result && result.rgba instanceof Uint8Array) {
      return [result.rgba.buffer as ArrayBuffer]
    }
    return []
  }
}

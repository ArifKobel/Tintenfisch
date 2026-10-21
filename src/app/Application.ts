import { CommentsStore } from '@/stores/CommentsStore.ts'
import { DocumentStore } from '@/stores/DocumentStore.ts'
import { EditingStore } from '@/stores/EditingStore.ts'
import { EngineClient, type EnginePort } from '@/services/EngineClient.ts'
import { AutoSave } from '@/stores/AutoSave.ts'
import { DocumentLibrary } from '@/services/DocumentLibrary.ts'
import { FilesStore } from '@/stores/FilesStore.ts'
import { Router, type BrowserWindow } from '@/services/Router.ts'
import { FindStore } from '@/stores/FindStore.ts'
import { FontCatalog, type CatalogSnapshot } from '@/services/FontCatalog.ts'
import { FontConsent, type ConsentStorage } from '@/stores/FontConsent.ts'
import { FontReplacements } from '@/stores/FontReplacements.ts'
import { FormsStore } from '@/stores/FormsStore.ts'
import { ImagesStore } from '@/stores/ImagesStore.ts'
import { BookmarksStore } from '@/stores/BookmarksStore.ts'
import { LinksStore } from '@/stores/LinksStore.ts'
import { TargetFollower } from '@/stores/TargetFollower.ts'
import { OrganizeStore } from '@/stores/OrganizeStore.ts'
import { CanvasPictureEncoder } from '@/services/CanvasPictureEncoder.ts'
import { PrintStore } from '@/stores/PrintStore.ts'
import { ProtectionStore } from '@/stores/ProtectionStore.ts'
import { RedactionStore } from '@/stores/RedactionStore.ts'
import { TextSelectionStore } from '@/stores/TextSelectionStore.ts'
import { PanelStore } from '@/stores/PanelStore.ts'
import { DigitalSignatureStore } from '@/stores/DigitalSignatureStore.ts'
import { SignatureStore } from '@/stores/SignatureStore.ts'
import { EDITING_TOOLS, ToolStore } from '@/stores/ToolStore.ts'
import { PageRevisionStore } from '@/stores/PageRevisionStore.ts'
import { TileCache } from '@/lib/TileCache.ts'
import { TileLoader } from '@/services/TileLoader.ts'
import { ViewportStore } from '@/stores/ViewportStore.ts'

export interface Browser {
  indexedDB: IDBFactory
  window: BrowserWindow
  storage: ConsentStorage | null
  fetch: typeof fetch
}

export class Application {
  readonly engine: EngineClient
  readonly documents: DocumentStore
  readonly viewport: ViewportStore
  readonly tiles: TileLoader
  readonly selection: TextSelectionStore
  readonly find: FindStore
  readonly revisions: PageRevisionStore
  readonly editing: EditingStore
  readonly organize: OrganizeStore
  readonly tools: ToolStore
  readonly panels: PanelStore
  readonly comments: CommentsStore
  readonly forms: FormsStore
  readonly images: ImagesStore
  readonly redaction: RedactionStore
  readonly printing: PrintStore
  readonly bookmarks: BookmarksStore
  readonly links: LinksStore
  readonly protection: ProtectionStore
  readonly signing: SignatureStore
  readonly digitalSignatures: DigitalSignatureStore
  readonly autoSave: AutoSave
  readonly files: FilesStore
  readonly fontConsent: FontConsent
  readonly fontReplacements: FontReplacements
  readonly fontCatalog: FontCatalog

  constructor(port: EnginePort, browser: Browser) {
    this.engine = new EngineClient(port)
    this.documents = new DocumentStore(this.engine)
    this.viewport = new ViewportStore()
    this.tiles = new TileLoader(this.engine, new TileCache())
    this.selection = new TextSelectionStore(this.engine)
    this.find = new FindStore(this.engine)
    this.revisions = new PageRevisionStore()
    this.editing = new EditingStore(this.engine, this.revisions)
    this.organize = new OrganizeStore(this.engine, this.documents, this.revisions)
    this.tools = new ToolStore()
    this.panels = new PanelStore()
    this.comments = new CommentsStore(this.engine, this.tools, this.revisions, this.panels)
    this.forms = new FormsStore(this.engine, this.revisions)
    this.images = new ImagesStore(this.engine, this.revisions)
    this.redaction = new RedactionStore(this.engine, this.revisions)
    const follower = new TargetFollower(this.viewport)
    this.bookmarks = new BookmarksStore(this.engine, follower)
    this.links = new LinksStore(this.engine, this.revisions, follower)
    this.protection = new ProtectionStore(this.engine)
    this.printing = new PrintStore(this.engine, this.documents, this.viewport, this.editing, CanvasPictureEncoder.encode)
    this.signing = new SignatureStore(this.engine, this.revisions)
    this.digitalSignatures = new DigitalSignatureStore(this.engine, this.documents)
    const library = new DocumentLibrary(browser.indexedDB)
    this.autoSave = new AutoSave(this.engine, this.documents, library)
    this.files = new FilesStore(library, this.documents, new Router(browser.window), this.autoSave)
    this.fontConsent = new FontConsent(browser.storage)
    const received = async (url: string) => {
      const response = await browser.fetch(url)
      if (!response.ok) throw new Error(`${url} answered ${response.status}`)
      return response
    }
    this.fontCatalog = new FontCatalog(async () => (await (await received('/fonts/catalog.json')).json()) as CatalogSnapshot)
    this.fontReplacements = new FontReplacements(this.engine, this.editing, this.fontConsent, this.fontCatalog, async (url) => new Uint8Array(await (await received(url)).arrayBuffer()))
    let previousTool = this.tools.getSnapshot().tool
    this.tools.subscribe(() => {
      const { tool } = this.tools.getSnapshot()
      this.editing.setEnabled(tool === 'edit' || tool === 'addText')
      if (tool !== 'edit') this.images.select(null)
      if (tool !== 'link') this.links.select(null)
      if (tool === 'sign' && previousTool !== 'sign' && !this.signing.getSnapshot().mark) this.signing.openPad()
      if (EDITING_TOOLS.includes(tool)) this.panels.showLeft('edit')
      else if (this.panels.getSnapshot().left === 'edit') this.panels.showLeft('tools')
      previousTool = tool
    })
    this.panels.subscribe(() => {
      const editing = EDITING_TOOLS.includes(this.tools.getSnapshot().tool)
      const editPanel = this.panels.getSnapshot().left === 'edit'
      if (editPanel && !editing) this.tools.choose('edit')
      if (!editPanel && editing) this.tools.choose('select')
    })
    this.documents.onDocumentReplaced(() => {
      this.editing.reset()
      this.organize.reset()
      this.comments.reset()
      this.forms.reset()
      this.images.reset()
      this.bookmarks.reset()
      this.links.reset()
      this.protection.reset()
      this.redaction.reset()
      this.digitalSignatures.reset()
      if (this.documents.getSnapshot().phase !== 'ready') return
      void this.comments.reload()
      void this.forms.reload()
      void this.bookmarks.reload()
      void this.digitalSignatures.reload()
    })
    this.editing.onDocumentChange(() => {
      void this.find.refresh()
      void this.bookmarks.reload()
      void this.documents.refreshPages()
      void this.comments.reload()
      void this.forms.reload()
    })
    this.organize.onDocumentChange(() => {
      void this.find.refresh()
      void this.bookmarks.reload()
      void this.comments.reload()
      void this.forms.reload()
    })
    this.signing.onDocumentChange(() => void this.forms.reload())
    this.redaction.onDocumentChange(() => {
      void this.find.refresh()
      void this.comments.reload()
      void this.forms.reload()
    })
  }
}

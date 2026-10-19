import { CommentsStore } from '@comments/CommentsStore.ts'
import { DocumentStore } from '@document/DocumentStore.ts'
import { EditingStore } from '@editing/EditingStore.ts'
import { EngineClient, type EnginePort } from '@engine/EngineClient.ts'
import { FindStore } from '@find/FindStore.ts'
import { FormsStore } from '@forms/FormsStore.ts'
import { ImagesStore } from '@images/ImagesStore.ts'
import { BookmarksStore } from '@navigation/BookmarksStore.ts'
import { LinksStore } from '@navigation/LinksStore.ts'
import { TargetFollower } from '@navigation/TargetFollower.ts'
import { OrganizeStore } from '@organize/OrganizeStore.ts'
import { CanvasPictureEncoder } from '@printing/CanvasPictureEncoder.ts'
import { PrintStore } from '@printing/PrintStore.ts'
import { ProtectionStore } from '@protection/ProtectionStore.ts'
import { RedactionStore } from '@redaction/RedactionStore.ts'
import { TextSelectionStore } from '@selection/TextSelectionStore.ts'
import { PanelStore } from '@shell/PanelStore.ts'
import { DigitalSignatureStore } from '@signing/DigitalSignatureStore.ts'
import { SignatureStore } from '@signing/SignatureStore.ts'
import { EDITING_TOOLS, ToolStore } from '@tools/ToolStore.ts'
import { PageRevisionStore } from '@viewer/PageRevisionStore.ts'
import { TileCache } from '@viewer/TileCache.ts'
import { TileLoader } from '@viewer/TileLoader.ts'
import { ViewportStore } from '@viewer/ViewportStore.ts'

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

  constructor(port: EnginePort) {
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
    let previousTool = this.tools.getSnapshot().tool
    this.tools.subscribe(() => {
      const { tool } = this.tools.getSnapshot()
      this.editing.setEnabled(tool === 'edit' || tool === 'addText')
      if (tool !== 'edit') this.images.select(null)
      if (tool !== 'link') this.links.select(null)
      if (tool === 'sign' && previousTool !== 'sign' && !this.signing.getSnapshot().mark) this.signing.openPad()
      if (EDITING_TOOLS.includes(tool)) this.panels.showLeft('edit')
      else if (this.panels.getSnapshot().left === 'edit') this.panels.showLeft(null)
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

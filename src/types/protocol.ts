export type Priority = 'interactive' | 'visible' | 'prefetch'

export interface Opened {
  version: string
  byteLength: number
  objectCount: number
  pageCount: number
  repaired: boolean
}

export type OpenResult = ({ status: 'opened' } & Opened) | { status: 'locked'; wrongPassword: boolean }

export interface PageSummary {
  id: string
  width: number
  height: number
  rotate: number
}

export interface TextPoint {
  line: number
  offset: number
}

export interface Highlight {
  x: number
  y: number
  width: number
  height: number
  angle: number
}

export type SelectionUnit = 'character' | 'word' | 'paragraph'

export interface TextSelection {
  anchor: TextPoint
  focus: TextPoint
  rects: Highlight[]
  text: string
}

export interface FindMatch {
  pageId: string
  rects: Highlight[]
}

export type BundledFamily = 'sans' | 'serif' | 'mono'

export type FontChoice =
  | { kind: 'document'; name: string }
  | { kind: 'bundled'; family: BundledFamily }
  | { kind: 'catalog'; key: string; source: 'fontsource' | 'fontshare'; id: string; family: string; weight: number; italic: boolean }

export interface TextStyle {
  font?: FontChoice
  bold?: boolean
  italic?: boolean
  size?: number
  color?: Rgb
}

export interface StyledRange extends TextStyle {
  start: number
  end: number
}

export interface ClusterStyle {
  resource: string | null
  font: string
  size: number
  bold: boolean
  italic: boolean
  color: Rgb | null
}

export interface ParagraphText {
  line: number
  clusters: string[]
  start: number
  styles: ClusterStyle[]
}

export interface PageFont {
  name: string
  font: string | null
  baseFont: string
  embedded: boolean
  family: BundledFamily
  bold: boolean
  italic: boolean
  weight: number | null
}

export type ClusterRange = [number, number]

export interface EditDraft {
  clusters: string[]
  caret: number
  dx: number
  dy: number
  width: number | null
  styles: StyledRange[]
  selection: ClusterRange | null
}

export interface NewTextDraft {
  x: number
  y: number
  text: string
  caret: number
  selection: ClusterRange | null
  family: BundledFamily
  catalog: string | null
  bold: boolean
  italic: boolean
  size: number
  color: Rgb
}

export interface EditPreview {
  caret: Highlight | null
  frame: Highlight
  selection: Highlight[]
}

export type Target = { kind: 'page'; pageId: string; top: number | null } | { kind: 'uri'; uri: string }

export interface Bookmark {
  id: string
  title: string
  target: Target | null
  open: boolean
  children: Bookmark[]
}

export interface DocumentLink {
  id: string
  rect: Box
  target: Target | null
}

export interface AddedLink {
  id: string
  dirtyPages: string[]
}

export interface Permissions {
  print: boolean
  printFaithful: boolean
  modify: boolean
  copy: boolean
  annotate: boolean
  fillForms: boolean
  assemble: boolean
}

export interface ProtectionState {
  encrypted: boolean
  ownerRights: boolean
  permissions: Permissions
}

export interface InsertedImage {
  index: number
  dirtyPages: string[]
}

export interface DocumentChange {
  dirtyPages: string[]
  uiState?: string | null
}

export interface PagesChange {
  pages: PageSummary[]
  dirtyPages: string[]
}

export type InsertResult = { status: 'inserted'; pages: PageSummary[]; inserted: string[] } | { status: 'locked'; wrongPassword: boolean }

export type Point = [number, number]

export type MarkupRequest =
  | { kind: 'highlight'; anchor: [number, number]; focus: [number, number] }
  | { kind: 'note'; at: Point; text: string }
  | { kind: 'ink'; strokes: Point[][] }
  | { kind: 'rectangle'; from: Point; to: Point }
  | { kind: 'ellipse'; from: Point; to: Point }
  | { kind: 'arrow'; from: Point; to: Point }
  | { kind: 'stamp'; at: Point; label: string }
  | { kind: 'textBox'; from: Point; to: Point; text: string }
  | { kind: 'redactArea'; from: Point; to: Point }
  | { kind: 'redactText'; anchor: [number, number]; focus: [number, number] }

export type Rgb = [number, number, number]

export interface SignatureInfo {
  fieldId: string
  fieldName: string
  signer: string | null
  signedAt: string
  reason: string
  status: 'valid' | 'broken' | 'unsupported'
  problem: string | null
  changedAfter: boolean
}

export interface SigningRequest {
  fieldId: string
  certificate: Uint8Array
  password: string
  reason: string
  location: string
}

export type SignatureMarkRequest =
  | { kind: 'ink'; strokes: Point[][]; penWidth: number; color: Rgb }
  | { kind: 'image'; width: number; height: number; rgba: Uint8Array }

export interface MarkupStyleRequest {
  color: Rgb
  width: number
  author: string
  date: string
}

export interface Comment {
  id: string
  pageId: string
  subtype: string
  contents: string
  author: string
  date: string
  rect: { x: number; y: number; width: number; height: number }
  color: Rgb | null
}

export interface AddedComment {
  id: string
  dirtyPages: string[]
}

export type FieldKind = 'text' | 'checkbox' | 'radio' | 'button' | 'combo' | 'list' | 'signature'

export interface FormWidget {
  id: string
  pageId: string
  rect: { x: number; y: number; width: number; height: number }
  onState: string | null
}

export interface FormField {
  id: string
  name: string
  partialName: string
  kind: FieldKind
  multiline: boolean
  password: boolean
  maxLength: number | null
  editable: boolean
  readOnly: boolean
  required: boolean
  text: string
  on: string | null
  chosen: string[]
  options: { value: string; label: string }[]
  widgets: FormWidget[]
}

export type NewFieldKind = 'text' | 'multiline' | 'checkbox' | 'radio' | 'combo' | 'list'

export interface Box {
  x: number
  y: number
  width: number
  height: number
}

export interface NewFieldRequest {
  kind: NewFieldKind
  name: string
  rect: Box
  options: string[]
}

export interface FieldProperties {
  name: string
  required: boolean
  readOnly: boolean
  multiline: boolean
  options: string[]
}

export interface AddedField {
  fieldId: string
  dirtyPages: string[]
}

export type FieldInput =
  | { kind: 'text'; text: string }
  | { kind: 'checked'; checked: boolean }
  | { kind: 'radio'; widget: string }
  | { kind: 'choices'; choices: string[] }

export interface TileRequest {
  pageId: string
  scale: number
  x: number
  y: number
  width: number
  height: number
}

export interface RenderedTile {
  width: number
  height: number
  rgba: Uint8Array
}

export type Command =
  | { kind: 'open'; bytes: Uint8Array }
  | { kind: 'unlock'; password: string }
  | { kind: 'pages' }
  | ({ kind: 'renderTile' } & TileRequest)
  | { kind: 'save'; optimize: boolean }
  | { kind: 'hitText'; pageId: string; x: number; y: number }
  | { kind: 'hitDraft'; pageId: string; x: number; y: number }
  | { kind: 'textBlocks'; pageId: string }
  | { kind: 'selectText'; pageId: string; anchor: TextPoint; focus: TextPoint; unit: SelectionUnit }
  | { kind: 'find'; query: string }
  | { kind: 'paragraphText'; pageId: string; line: number }
  | { kind: 'previewEdit'; pageId: string; line: number; draft: EditDraft }
  | { kind: 'previewNewText'; pageId: string; draft: NewTextDraft }
  | { kind: 'pageFonts'; pageId: string }
  | { kind: 'registerFontReplacement'; key: string; bytes: Uint8Array }
  | { kind: 'images'; pageId: string }
  | { kind: 'insertImage'; pageId: string; bytes: Uint8Array; x: number; y: number }
  | { kind: 'moveImage'; pageId: string; index: number; frame: Highlight }
  | { kind: 'replaceImage'; pageId: string; index: number; bytes: Uint8Array }
  | { kind: 'deleteImage'; pageId: string; index: number }
  | { kind: 'outline' }
  | { kind: 'protection' }
  | { kind: 'grantOwnerRights'; password: string }
  | { kind: 'protect'; userPassword: string; ownerPassword: string; permissions: Permissions; entropy: Uint8Array }
  | { kind: 'unprotect' }
  | { kind: 'addBookmark'; parent: string | null; index: number; title: string; target: Target | null }
  | { kind: 'renameBookmark'; bookmarkId: string; title: string }
  | { kind: 'retargetBookmark'; bookmarkId: string; target: Target }
  | { kind: 'moveBookmark'; bookmarkId: string; parent: string | null; index: number }
  | { kind: 'deleteBookmark'; bookmarkId: string }
  | { kind: 'links'; pageId: string }
  | { kind: 'addLink'; pageId: string; rect: Box; target: Target }
  | { kind: 'updateLink'; pageId: string; linkId: string; rect: Box; target: Target }
  | { kind: 'commitEdit'; uiState: string }
  | { kind: 'cancelEdit' }
  | { kind: 'rotatePages'; pageIds: string[]; quarterTurns: number }
  | { kind: 'deletePages'; pageIds: string[] }
  | { kind: 'movePages'; pageIds: string[]; before: string | null }
  | { kind: 'insertBlankPage'; before: string | null; width: number; height: number }
  | { kind: 'insertDocument'; bytes: Uint8Array; password: string; before: string | null }
  | { kind: 'extractPages'; pageIds: string[] }
  | { kind: 'comments' }
  | { kind: 'addMarkup'; pageId: string; markup: MarkupRequest; style: MarkupStyleRequest }
  | { kind: 'deleteComment'; pageId: string; commentId: string }
  | { kind: 'setCommentContents'; commentId: string; contents: string; date: string }
  | { kind: 'applyRedactions' }
  | { kind: 'fields' }
  | { kind: 'setField'; fieldId: string; input: FieldInput }
  | { kind: 'addField'; pageId: string; field: NewFieldRequest }
  | { kind: 'setFieldProperties'; fieldId: string; properties: FieldProperties }
  | { kind: 'moveWidget'; pageId: string; widgetId: string; rect: Box }
  | { kind: 'deleteField'; fieldId: string }
  | { kind: 'addSignature'; pageId: string; rect: Box; name: string; mark: SignatureMarkRequest }
  | ({ kind: 'signField'; date: string } & SigningRequest)
  | { kind: 'signatures' }
  | { kind: 'undo' }
  | { kind: 'redo' }

export interface Results {
  open: OpenResult
  unlock: OpenResult
  pages: PageSummary[]
  renderTile: RenderedTile
  save: Uint8Array
  hitText: TextPoint | null
  hitDraft: number | null
  textBlocks: Highlight[]
  selectText: TextSelection
  find: FindMatch[]
  paragraphText: ParagraphText
  previewEdit: EditPreview
  previewNewText: EditPreview
  pageFonts: PageFont[]
  registerFontReplacement: void
  images: Highlight[]
  insertImage: InsertedImage
  moveImage: DocumentChange
  replaceImage: DocumentChange
  deleteImage: DocumentChange
  outline: Bookmark[]
  protection: ProtectionState
  grantOwnerRights: boolean
  protect: undefined
  unprotect: undefined
  addBookmark: string
  renameBookmark: undefined
  retargetBookmark: undefined
  moveBookmark: undefined
  deleteBookmark: undefined
  links: DocumentLink[]
  addLink: AddedLink
  updateLink: DocumentChange
  commitEdit: DocumentChange
  cancelEdit: undefined
  rotatePages: PagesChange
  deletePages: PagesChange
  movePages: PagesChange
  insertBlankPage: PagesChange
  insertDocument: InsertResult
  extractPages: Uint8Array
  comments: Comment[]
  addMarkup: AddedComment
  deleteComment: DocumentChange
  setCommentContents: DocumentChange
  applyRedactions: DocumentChange
  fields: FormField[]
  setField: DocumentChange
  addField: AddedField
  setFieldProperties: DocumentChange
  moveWidget: DocumentChange
  deleteField: DocumentChange
  addSignature: AddedField
  signField: Uint8Array
  signatures: SignatureInfo[]
  undo: DocumentChange
  redo: DocumentChange
}

export interface Scheduling {
  priority: Priority
  group?: string
}

export type Request = Command & Scheduling & { id: number }

export interface Cancel {
  kind: 'cancel'
  ids: number[]
}

export type WorkerMessage = Request | Cancel

export type Response =
  | { id: number; ok: true; result: unknown; revision: string }
  | { id: number; ok: false; error: string; cancelled?: boolean }

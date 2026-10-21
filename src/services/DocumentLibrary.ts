export interface StoredFile {
  id: string
  name: string
  size: number
  opened: number
  modified: number
}

const DATABASE = 'tintenfisch-files'
const FILES = 'files'
const CONTENTS = 'contents'

function settled<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

function finished(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve()
    transaction.onerror = () => reject(transaction.error)
    transaction.onabort = () => reject(transaction.error)
  })
}

export class DocumentLibrary {
  readonly #database: Promise<IDBDatabase>
  readonly #now: () => number

  constructor(factory: IDBFactory, now: () => number = Date.now) {
    this.#now = now
    const opening = factory.open(DATABASE, 1)
    opening.onupgradeneeded = () => {
      opening.result.createObjectStore(FILES, { keyPath: 'id' })
      opening.result.createObjectStore(CONTENTS)
    }
    this.#database = settled(opening)
  }

  async list(): Promise<StoredFile[]> {
    const files = await this.#read((stores) => settled(stores.files.getAll() as IDBRequest<StoredFile[]>))
    return files.sort((left, right) => right.opened - left.opened)
  }

  async add(name: string, bytes: Uint8Array): Promise<StoredFile> {
    const now = this.#now()
    const file: StoredFile = { id: crypto.randomUUID(), name, size: bytes.byteLength, opened: now, modified: now }
    await this.#write((stores) => {
      stores.files.put(file)
      stores.contents.put(bytes, file.id)
    })
    return file
  }

  async open(id: string): Promise<{ file: StoredFile; bytes: Uint8Array } | null> {
    const [file, bytes] = await this.#read((stores) =>
      Promise.all([settled(stores.files.get(id) as IDBRequest<StoredFile | undefined>), settled(stores.contents.get(id) as IDBRequest<Uint8Array | undefined>)]),
    )
    if (!file || !bytes) return null
    const opened = { ...file, opened: this.#now() }
    await this.#write((stores) => stores.files.put(opened))
    return { file: opened, bytes }
  }

  async replace(id: string, bytes: Uint8Array): Promise<void> {
    const file = await this.#read((stores) => settled(stores.files.get(id) as IDBRequest<StoredFile | undefined>))
    if (!file) throw new Error('the document is no longer stored')
    await this.#write((stores) => {
      stores.files.put({ ...file, size: bytes.byteLength, modified: this.#now() })
      stores.contents.put(bytes, id)
    })
  }

  async remove(id: string): Promise<void> {
    await this.#write((stores) => {
      stores.files.delete(id)
      stores.contents.delete(id)
    })
  }

  async #read<T>(work: (stores: { files: IDBObjectStore; contents: IDBObjectStore }) => Promise<T>): Promise<T> {
    const transaction = (await this.#database).transaction([FILES, CONTENTS], 'readonly')
    return work({ files: transaction.objectStore(FILES), contents: transaction.objectStore(CONTENTS) })
  }

  async #write(work: (stores: { files: IDBObjectStore; contents: IDBObjectStore }) => void): Promise<void> {
    const transaction = (await this.#database).transaction([FILES, CONTENTS], 'readwrite')
    work({ files: transaction.objectStore(FILES), contents: transaction.objectStore(CONTENTS) })
    await finished(transaction)
  }
}

export const documentLibrary = new DocumentLibrary(indexedDB)

export const TRUE = 0x10
export const FALSE = 0x00

export function splitPath(path: string): number[] {
  const result: number[] = []
  const allComponents = path.trim().split('/')
  const components = allComponents.length > 0 && allComponents[0] == 'm' ? allComponents.slice(1) : allComponents
  components.forEach((element) => {
    let number = parseInt(element, 10)
    if (isNaN(number)) {
      throw Error(`Invalid bip32 path: ${path}`)
    }
    if (element.length > 1 && element[element.length - 1] === "'") {
      number += 0x80000000
    }
    result.push(number)
  })
  return result
}

export function serializePath(path: string): Buffer {
  const nodes = splitPath(path)

  if (nodes.length != 5) {
    throw Error('Invalid BIP32 path length')
  }
  const buffer = Buffer.alloc(nodes.length * 4)
  nodes.forEach((element, index) => buffer.writeUInt32BE(element, 4 * index))
  return buffer
}

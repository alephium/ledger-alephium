import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos'
import fetch from 'node-fetch'
import { sleep } from '@alephium/web3'
import Transport from '@ledgerhq/hw-transport'
import NodeTransport from '@ledgerhq/hw-transport-node-hid'

async function pressButton(button: 'left' | 'right' | 'both') {
  await sleep(1000)
  return fetch(`http://localhost:25000/button/${button}`, {
    method: 'POST',
    body: JSON.stringify({ action: 'press-and-release' })
  })
}

async function clickAndApprove(times: number) {
  for (let i = 0; i < times; i++) {
    await pressButton('right')
  }
  await pressButton('both')
}

export enum OutputType {
  Base,
  Multisig,
  Token,
  MultisigAndToken
}

const NanosClickTable = new Map([
  [OutputType.Base, 5],
  [OutputType.Multisig, 10],
  [OutputType.Token, 11],
  [OutputType.MultisigAndToken, 16],
])

const NanospClickTable = new Map([
  [OutputType.Base, 3],
  [OutputType.Multisig, 5],
  [OutputType.Token, 6],
  [OutputType.MultisigAndToken, 8],
])

const StaxClickTable = new Map([
  [OutputType.Base, 2],
  [OutputType.Multisig, 3],
  [OutputType.Token, 3],
  [OutputType.MultisigAndToken, 4],
])

function getOutputClickSize(outputType: OutputType) {
  const model = process.env.MODEL
  switch (model) {
    case 'nanos': return NanosClickTable.get(outputType)!
    case 'nanosp':
    case 'nanox': return NanospClickTable.get(outputType)!
    case 'stax':
    case 'flex': return StaxClickTable.get(outputType)!
    default: throw new Error(`Unknown model ${model}`)
  }
}

async function click(outputs: OutputType[], hasExternalInputs: boolean) {
  await sleep(1000);
  if (hasExternalInputs) {
    await clickAndApprove(1)
  }

  for (let index = 0; index < outputs.length; index += 1) {
    await clickAndApprove(getOutputClickSize(outputs[index]))
  }

  await clickAndApprove(1) // fees
}

interface Position {
  x: number
  y: number
}

const STAX_CONTINUE_POSITION = { x: 342, y: 606 }
const STAX_APPROVE_POSITION = { x: 200, y: 515 }
const STAX_REJECT_POSITION = { x: 36, y: 606 }
const STAX_SETTINGS_POSITION = { x: 342, y: 55 }
const STAX_BLIND_SETTING_POSITION = { x: 342, y: 90 }

const FLEX_CONTINUE_POSITION = { x: 430, y: 550 }
const FLEX_APPROVE_POSITION = { x: 240, y: 435 }
const FLEX_REJECT_POSITION = { x: 55, y: 530 }
const FLEX_SETTINGS_POSITION = { x: 405, y: 75 }
const FLEX_BLIND_SETTING_POSITION = { x: 405, y: 96 }

async function touchPosition(pos: Position) {
  await sleep(1000)
  return fetch(`http://localhost:25000/finger`, {
    method: 'POST',
    body: JSON.stringify({ action: 'press-and-release', x: pos.x, y: pos.y })
  })
}

async function _touch(times: number) {
  let continuePos = process.env.MODEL === 'stax' ? STAX_CONTINUE_POSITION : FLEX_CONTINUE_POSITION
  for (let i = 0; i < times; i += 1) {
    await touchPosition(continuePos)
  }
  let approvePos = process.env.MODEL === 'stax' ? STAX_APPROVE_POSITION : FLEX_APPROVE_POSITION
  await touchPosition(approvePos)
}

export async function staxFlexApproveOnce() {
  if (process.env.MODEL === 'stax') {
    await touchPosition(STAX_APPROVE_POSITION)
  } else {
    await touchPosition(FLEX_APPROVE_POSITION)
  }
}

async function touch(outputs: OutputType[], hasExternalInputs: boolean) {
  await sleep(1000);
  if (hasExternalInputs) {
    await staxFlexApproveOnce()
  }

  for (let index = 0; index < outputs.length; index += 1) {
    await _touch(getOutputClickSize(outputs[index]))
  }

  await _touch(2) // fees
}

export async function approveTx(outputs: OutputType[], hasExternalInputs: boolean = false) {
  if (!needToAutoApprove()) return
  const isSelfTransfer = outputs.length === 0 && !hasExternalInputs
  if (isSelfTransfer) {
    if (isStaxOrFlex()) {
      await _touch(2)
    } else {
      await clickAndApprove(2)
    }
    return
  }

  if (isStaxOrFlex()) {
    await touch(outputs, hasExternalInputs)
  } else {
    await click(outputs, hasExternalInputs)
  }
}

export async function approveHash() {
  if (!needToAutoApprove()) return
  if (isStaxOrFlex()) {
    return await _touch(3)
  }
  if (process.env.MODEL === 'nanos') {
    await clickAndApprove(5)
  } else {
    await clickAndApprove(3)
  }
}

export async function approveAddress() {
  if (!needToAutoApprove()) return
  if (isStaxOrFlex()) {
    return await _touch(2)
  }
  if (process.env.MODEL === 'nanos') {
    await clickAndApprove(4)
  } else {
    await clickAndApprove(2)
  }
}

function isStaxOrFlex(): boolean {
  return !process.env.MODEL!.startsWith('nano')
}

export function skipBlindSigningWarning() {
  if (!needToAutoApprove()) return
  if (isStaxOrFlex()) {
    const rejectPos = process.env.MODEL === 'stax' ? STAX_REJECT_POSITION : FLEX_REJECT_POSITION
    touchPosition(rejectPos)
  } else {
    clickAndApprove(3)
  }
}

export async function enableBlindSigning() {
  if (!needToAutoApprove()) return
  if (isStaxOrFlex()) {
    const settingsPos = process.env.MODEL === 'stax' ? STAX_SETTINGS_POSITION : FLEX_SETTINGS_POSITION
    const blindSettingPos = process.env.MODEL === 'stax' ? STAX_BLIND_SETTING_POSITION : FLEX_BLIND_SETTING_POSITION
    await touchPosition(settingsPos)
    await touchPosition(blindSettingPos)
    await touchPosition(settingsPos)
  } else {
    await clickAndApprove(2)
  }
}

export function getRandomInt(min: number, max: number) {
  min = Math.ceil(min)
  max = Math.floor(max)
  return Math.floor(Math.random() * (max - min) + min) // The maximum is exclusive and the minimum is inclusive
}

export function needToAutoApprove(): boolean {
  switch (process.env.BACKEND) {
    case "speculos": return true
    case "device": return false
    default: throw new Error(`Invalid backend: ${process.env.BACKEND}`)
  }
}

const ApduPort = 9999

export async function createTransport(): Promise<Transport> {
  switch (process.env.BACKEND) {
    case "speculos": return SpeculosTransport.open({ apduPort: ApduPort })
    case "device": return NodeTransport.open('')
    default: throw new Error(`Invalid backend: ${process.env.BACKEND}`)
  }
}

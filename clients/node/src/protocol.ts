import * as net from 'net';

export const PROTOCOL_MAGIC = Buffer.from('THRP', 'ascii');
export const PROTOCOL_VERSION = 1;
export const FRAME_HEADER_SIZE = 20;

export enum MessageType {
  Startup = 0x0001,
  AuthChallenge = 0x0002,
  AuthResponse = 0x0003,
  AuthOk = 0x0004,
  Close = 0x000E,
  Query = 0x0010,
  RowDescription = 0x0020,
  DataRow = 0x0021,
  CommandComplete = 0x0022,
  Error = 0x00FF,
}

export interface Frame {
  messageType: MessageType;
  requestId: bigint;
  payload: Buffer;
}

export function encodeFrame(frame: Frame): Buffer {
  const buf = Buffer.alloc(FRAME_HEADER_SIZE + frame.payload.length);
  PROTOCOL_MAGIC.copy(buf, 0, 0, 4);
  buf.writeUInt16LE(PROTOCOL_VERSION, 4);
  buf.writeUInt16LE(frame.messageType, 6);
  buf.writeUInt32LE(frame.payload.length, 8);
  buf.writeBigUInt64LE(frame.requestId, 12);
  frame.payload.copy(buf, FRAME_HEADER_SIZE);
  return buf;
}

export function decodeFrame(buf: Buffer): { frame: Frame; bytesRead: number } | null {
  if (buf.length < FRAME_HEADER_SIZE) return null;
  if (!buf.subarray(0, 4).equals(PROTOCOL_MAGIC)) {
    throw new Error('Invalid protocol magic bytes');
  }

  const payloadLen = buf.readUInt32LE(8);
  const totalLen = FRAME_HEADER_SIZE + payloadLen;
  if (buf.length < totalLen) return null;

  const msgType = buf.readUInt16LE(6);
  const reqId = buf.readBigUInt64LE(12);
  const payload = buf.subarray(FRAME_HEADER_SIZE, totalLen);

  return {
    frame: {
      messageType: msgType as MessageType,
      requestId: reqId,
      payload,
    },
    bytesRead: totalLen,
  };
}

export const PROTOCOL_MAGIC = Buffer.from('THRP', 'ascii');
export const PROTOCOL_VERSION = 1;
export const FRAME_HEADER_SIZE = 20;
export var MessageType;
(function (MessageType) {
    MessageType[MessageType["Startup"] = 1] = "Startup";
    MessageType[MessageType["AuthChallenge"] = 2] = "AuthChallenge";
    MessageType[MessageType["AuthResponse"] = 3] = "AuthResponse";
    MessageType[MessageType["AuthOk"] = 4] = "AuthOk";
    MessageType[MessageType["Close"] = 14] = "Close";
    MessageType[MessageType["Query"] = 16] = "Query";
    MessageType[MessageType["RowDescription"] = 32] = "RowDescription";
    MessageType[MessageType["DataRow"] = 33] = "DataRow";
    MessageType[MessageType["CommandComplete"] = 34] = "CommandComplete";
    MessageType[MessageType["Error"] = 255] = "Error";
})(MessageType || (MessageType = {}));
export function encodeFrame(frame) {
    const buf = Buffer.alloc(FRAME_HEADER_SIZE + frame.payload.length);
    PROTOCOL_MAGIC.copy(buf, 0, 0, 4);
    buf.writeUInt16LE(PROTOCOL_VERSION, 4);
    buf.writeUInt16LE(frame.messageType, 6);
    buf.writeUInt32LE(frame.payload.length, 8);
    buf.writeBigUInt64LE(frame.requestId, 12);
    frame.payload.copy(buf, FRAME_HEADER_SIZE);
    return buf;
}
export function decodeFrame(buf) {
    if (buf.length < FRAME_HEADER_SIZE)
        return null;
    if (!buf.subarray(0, 4).equals(PROTOCOL_MAGIC)) {
        throw new Error('Invalid protocol magic bytes');
    }
    const payloadLen = buf.readUInt32LE(8);
    const totalLen = FRAME_HEADER_SIZE + payloadLen;
    if (buf.length < totalLen)
        return null;
    const msgType = buf.readUInt16LE(6);
    const reqId = buf.readBigUInt64LE(12);
    const payload = buf.subarray(FRAME_HEADER_SIZE, totalLen);
    return {
        frame: {
            messageType: msgType,
            requestId: reqId,
            payload,
        },
        bytesRead: totalLen,
    };
}

export declare const PROTOCOL_MAGIC: any;
export declare const PROTOCOL_VERSION = 1;
export declare const FRAME_HEADER_SIZE = 20;
export declare enum MessageType {
    Startup = 1,
    AuthChallenge = 2,
    AuthResponse = 3,
    AuthOk = 4,
    Close = 14,
    Query = 16,
    RowDescription = 32,
    DataRow = 33,
    CommandComplete = 34,
    Error = 255
}
export interface Frame {
    messageType: MessageType;
    requestId: bigint;
    payload: Buffer;
}
export declare function encodeFrame(frame: Frame): Buffer;
export declare function decodeFrame(buf: Buffer): {
    frame: Frame;
    bytesRead: number;
} | null;

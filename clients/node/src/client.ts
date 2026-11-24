import * as net from 'net';
import { encodeFrame, decodeFrame, MessageType, Frame } from './protocol.js';

export interface QueryResult {
  columns: string[];
  rows: any[][];
  rowsAffected: number;
}

export class TheresavClient {
  private socket: net.Socket;
  private reqId = 1n;

  constructor(private host: string = '127.0.0.1', private port: number = 7432) {
    this.socket = new net.Socket();
  }

  public async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.socket.connect(this.port, this.host, () => resolve());
      this.socket.on('error', (err: Error) => reject(err));
    });
  }

  public async query(sql: string): Promise<QueryResult> {
    const currentId = this.reqId++;
    const payload = Buffer.from(JSON.stringify({ sql }), 'utf8');
    const reqFrame = encodeFrame({
      messageType: MessageType.Query,
      requestId: currentId,
      payload,
    });

    this.socket.write(reqFrame);

    return new Promise((resolve, reject) => {
      let buffer = Buffer.alloc(0);
      const columns: string[] = [];
      const rows: any[][] = [];

      const onData = (chunk: Buffer) => {
        buffer = Buffer.concat([buffer, chunk]);

        while (buffer.length > 0) {
          const res = decodeFrame(buffer);
          if (!res) break;

          buffer = buffer.subarray(res.bytesRead);
          const { frame } = res;

          if (frame.messageType === MessageType.RowDescription) {
            const desc = JSON.parse(frame.payload.toString('utf8'));
            columns.push(...desc.columns);
          } else if (frame.messageType === MessageType.DataRow) {
            const rowMsg = JSON.parse(frame.payload.toString('utf8'));
            rows.push(rowMsg.values);
          } else if (frame.messageType === MessageType.CommandComplete) {
            const comp = JSON.parse(frame.payload.toString('utf8'));
            this.socket.off('data', onData);
            resolve({
              columns,
              rows,
              rowsAffected: comp.rows_affected,
            });
            return;
          } else if (frame.messageType === MessageType.Error) {
            const err = JSON.parse(frame.payload.toString('utf8'));
            this.socket.off('data', onData);
            reject(new Error(`[${err.code}] ${err.message}`));
            return;
          }
        }
      };

      this.socket.on('data', onData);
    });
  }

  public close(): void {
    this.socket.end();
  }
}

export interface QueryResult {
    columns: string[];
    rows: any[][];
    rowsAffected: number;
}
export declare class TheresavClient {
    private host;
    private port;
    private socket;
    private reqId;
    constructor(host?: string, port?: number);
    connect(): Promise<void>;
    query(sql: string): Promise<QueryResult>;
    close(): void;
}

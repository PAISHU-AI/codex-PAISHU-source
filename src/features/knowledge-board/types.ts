export interface KnowledgeDocumentSummary {
  id: string;
  title: string;
  sourceUri: string;
  owner: string;
  status: string;
  accessTier: string;
  enabled: boolean;
  chunkCount: number;
  approximateTokens: number;
  updatedAt: string;
}

export interface KnowledgeBoardData {
  refreshedAt: string;
  serviceStatus: string;
  collectionCount: number;
  totalDocuments: number;
  enabledDocuments: number;
  disabledDocuments: number;
  chunkCount: number;
  databaseBytes: number;
  averageReadMs: number;
  readSuccessCount: number;
  readFailureCount: number;
  documents: KnowledgeDocumentSummary[];
  messages: string[];
}

export interface KnowledgeOverview {
  documentId: string;
  title: string;
  language: string;
  overview: string;
  sourceUri: string;
  updatedAt: string;
}

export interface RadioStream {
  url: string;
  format: 'hls' | 'direct';
  quality?: string;
}

export interface RadioSource {
  id: string;
  name: string;
  logo?: string;
  category?: string;
  region?: string;
  description?: string;
  streams: RadioStream[];
}

export interface RadioCatalog {
  version: string;
  platform: string;
  updatedAt: string;
  sources: RadioSource[];
}

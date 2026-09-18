export type Device = {
  key: string,
  name: string,
  nickname: string | null,
  target: number,
  locked: boolean,
  connected: boolean,
  adjustable: boolean,
  isDefault: boolean,
  level: number | null,
};

export type Candidate = {
  key: string,
  name: string,
};

export type IdentifyPrompt =
  | { kind: "choose", candidates: Candidate[] }
  | { kind: "keepOnlyOne", keep: string | null };

export type UnidentifiedDevice = {
  endpoint: string,
  name: string,
  level: number | null,
  prompt: IdentifyPrompt,
};

export type DevicesSnapshot = {
  devices: Device[],
  unidentified: UnidentifiedDevice[],
};

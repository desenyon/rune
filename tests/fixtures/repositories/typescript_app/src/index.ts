export type TokenKind = "access" | "refresh";

export interface IssuedToken {
  kind: TokenKind;
  value: string;
}

export function issueToken(kind: TokenKind, value: string): IssuedToken {
  if (!value) {
    throw new Error("token value is required");
  }
  return { kind, value };
}

export function isRefresh(token: IssuedToken): boolean {
  return token.kind === "refresh";
}

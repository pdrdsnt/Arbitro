import { AnyPool } from "../pool/AnyPool";

export interface IFactory {
  searchPair(a: string, b: string): Promise<AnyPool | { error: string }>[];
}

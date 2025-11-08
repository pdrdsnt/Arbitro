import { MetaMaskInpageProvider } from "@metamask/providers";
import { V2Pool } from "../pool/V2Pool";
import { IFactory } from "./IFactroy";
import { Contract, ethers } from "ethers";
import { AnyPool } from "../pool/AnyPool";
import { Eip1193Provider } from "ethers";

const V2_FACTORY_ABI = [
  "function getPair(address tokenA, address tokenB) external view returns (address pair)",
];

export class V2Factory {
  name: string;
  address: string;
  fee: number;
  stable_fee: number | undefined;
  contract: V2FactoryContract | undefined;

  constructor(json_data: any) {
    // console.log("creating v2");
    this.name = json_data.name;
    this.address = json_data.address;
    this.fee = json_data.fee;
    this.stable_fee = json_data.stable_fee;
  }
}
//=================================================
//==================================================
export class V2FactoryContract implements IFactory {
  data: V2Factory;
  contract: Contract;
  found: Map<string, V2Pool>;

  constructor(
    data: V2Factory,
    eth: Eip1193Provider,
    pools_maybe_from_here: AnyPool[],
  ) {
    //console.log("creating v2");
    this.contract = new Contract(
      data.address,
      V2_FACTORY_ABI,
      new ethers.BrowserProvider(eth),
    );

    this.found = new Map();

    for (const pool of pools_maybe_from_here) {
      const p = pool as V2Pool;
      if (p) {
        if (p.factory === this.data.address) {
          const key = "" + p.token0 + p.token1;
          this.found.set(key, p);
        }
      }
    }
  }

  searchPair(a: string, b: string): Promise<AnyPool | { error: string }>[] {
    console.log("v2 searching ");
    if (this.contract === null) {
      return [];
    }

    let pair = [a, b].sort();
    let pair_key = "" + pair[0] + pair[1];
    let saved_pool = this.found.get(pair_key);

    if (saved_pool) {
      return [Promise.resolve(saved_pool)];
    }

    let calls: Promise<AnyPool | { error: string }>[] = [];
    let call = this.contract
      .getPair(pair[0], pair[1])
      .catch((err) => {
        console.log(err);
        return { error: err };
      })
      .then((pool) => {
        if (pool) {
          let new_pool = new V2Pool(
            pool,
            pair[0],
            pair[1],
            this.data.fee,
            this.data.address,
          );
          this.found.set(pair_key, new_pool);
          return new_pool as AnyPool;
        }
        return { error: "" };
      });

    calls.push(call);
    return calls;
  }
}

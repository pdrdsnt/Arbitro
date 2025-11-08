import { Contract, ethers } from "ethers";
import { IFactory } from "./IFactroy";
import { V3Pool } from "../pool/V3Pool";
import { AnyPool } from "../pool/AnyPool";
import { MetaMaskInpageProvider } from "@metamask/providers";
import { Eip1193Provider } from "ethers";

const V3_FACTORY_ABI = [
  "function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)",
];

export class V3Factory {
  name: string;
  address: string;
  fee: Array<number>;
  contract: V3FactoryContract | undefined;

  constructor(json_data: any) {
    //    console.log("creating v3");
    this.name = json_data.fee;
    this.address = json_data.address;
    this.fee = json_data.fee;
  }
}
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//=========================================================
const fee_spacing_map = {
  100: 1,
  250: 5,
  500: 10,
  1000: 20,
  1500: 30,
  2000: 40,
  2500: 50,
  3000: 60,
  5000: 100,
  10000: 200,
};
export class V3FactoryContract implements IFactory {
  data: V3Factory;
  found: Map<string, V3Pool>;
  contract: Contract;

  constructor(
    data: V3Factory,
    eth: Eip1193Provider,
    pools_maybe_from_here: AnyPool[],
  ) {
    this.data = data;
    //console.log("creating v3");
    this.contract = new Contract(
      data.address,
      V3_FACTORY_ABI,
      new ethers.BrowserProvider(eth),
    );

    this.found = new Map();

    for (const pool of pools_maybe_from_here) {
      const p = pool as V3Pool;
      if (p) {
        if (p.factory == this.data.address) {
          const key = "" + p.token0 + p.token1;
          this.found.set(key, p);
        }
      }
    }
  }

  searchPair(a: string, b: string): Promise<AnyPool | { error: string }>[] {
    console.log("v3 searching ");

    let calls: Promise<AnyPool | { error: string }>[] = [];
    if (this.contract === null) {
      return calls;
    }
    let pair = [a, b].sort();

    for (const fee of this.data.fee) {
      console.log("fee: ", fee);
      let key = pair[0] + pair[1] + fee;

      console.log("v3 key now: ", key);

      if (this.found.has(key)) {
        let pol = this.found.get(key);
        console.log("has key for cache data: ", pol);
        if (pol) {
          calls.push(Promise.resolve(pol));
        }
      }

      console.log("v3 done until now");

      const call: Promise<AnyPool | { error: string }> = this.contract
        .getPool(a, b, fee)
        .catch((err) => {
          return { error: err };
        })
        .then((pool) => {
          console.log("v3 promisse resolved for: ", pool);
          if (pool && pool !== ethers.ZeroAddress) {
            const new_pool = new V3Pool(
              pool,
              a,
              b,
              fee,
              fee_spacing_map[fee] ? fee_spacing_map[fee] : 100,
              this.data.address,
            );
            this.found.set(key, new_pool);
            let p = new_pool as AnyPool;
            return p;
          } else {
            return { error: "" };
          }
        });
      calls.push(call);
    }

    return calls;
  }
}

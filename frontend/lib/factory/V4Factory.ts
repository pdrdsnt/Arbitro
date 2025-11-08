import { Contract, ethers } from "ethers";
import { IFactory } from "./IFactroy";
import { AbiCoder } from "ethers";
import { keccak256 } from "ethers";
import { MetaMaskInpageProvider } from "@metamask/providers";
import { V4Pool } from "../pool/V4Pool";
import { AnyPool } from "../pool/AnyPool";
import { Eip1193Provider } from "ethers";

const V4_FACTORY_ABI = [
  "function liquidity(address pool) external view returns (uint256)",
];

export class V4Factory {
  name: string;
  address: string;
  found: Map<string, V4Pool>;
  contract: V4FactoryContract | undefined;

  constructor(json_data: any) {
    //console.log("creating v4");
    this.name = json_data.name;
    this.address = json_data.address;
    this.found = new Map();
  }
}
//
//=========================================================
//=========================================================
//=========================================================
//=========================================================
//
export class V4FactoryContract implements IFactory {
  data: V4Factory;
  contract: Contract;
  found: Map<string, V4Pool>;

  constructor(
    data: V4Factory,
    eth: Eip1193Provider,
    pools_maybe_from_here: AnyPool[],
  ) {
    this.data = this.data;
    this.contract = new Contract(
      data.address,
      V4_FACTORY_ABI,
      new ethers.BrowserProvider(eth),
    );

    this.found = new Map();

    for (const pool of pools_maybe_from_here) {
      const p = pool as V4Pool;
      if (p) {
        if (p.address == this.data.address) {
          const key = "" + p.token0 + p.token1;
          this.found.set(key, p);
        }
      }
    }
  }

  searchPair(a: string, b: string): Promise<AnyPool | { error: string }>[] {
    console.log("v4 searching ");
    let calls: Promise<AnyPool | { error: string }>[] = [];
    if (this.contract === null) {
      return [];
    }
    const pools: AnyPool[] = [];
    const fees = [1, 5, 20, 100, 500, 1000, 2000, 10000];
    const spacings = [1, 2, 5, 10, 20, 50, 80];
    const hooks: string[] = ["0x0000000000000000000000000000000000000000"]; // extend later

    for (const fee of fees) {
      for (const spacing of spacings) {
        for (const hook of hooks) {
          // encode PoolKey
          const encoded = AbiCoder.defaultAbiCoder().encode(
            ["address", "address", "uint24", "int24", "address"],
            [a, b, fee, spacing, hook],
          );

          const poolKey = keccak256(encoded);

          if (this.found.has(poolKey)) {
            let pool = this.found.get(poolKey);
            if (pool) pools.push(pool);
            continue;
          }

          const call = this.contract
            .liquidity(poolKey)
            .then((liquidty: bigint) => {
              if (liquidty) {
                const new_pool = new V4Pool(
                  this.data.address,
                  poolKey,
                  a,
                  b,
                  fee,
                  spacing,
                  hook,
                  liquidty,
                );
                this.found.set(poolKey, new_pool);
                return new_pool;
              } else {
                return { error: "" };
              }
            })
            .catch((err) => {
              console.log(err);
              return { error: err };
            });

          calls.push(call);
        }
      }
    }
    return calls;
  }
}

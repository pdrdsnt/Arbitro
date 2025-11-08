import { AnyFactory, FactoryBuilder } from "../factory/AnyFactory";
import { AnyPool, PoolBuilder } from "../pool/AnyPool";
import { ethers, getAddress, isAddress } from "ethers";

export type Token = { symbol: string; address: string; decimals: number };

export class ChainData {
  id: number;
  tokens: Map<string, Token> = new Map();
  pools_by_token: Map<string, AnyPool[]> = new Map();
  pools: Array<AnyPool> = [];
  factories: Array<AnyFactory> = [];

  constructor(chain_data: any, id: number) {
    if (chain_data === undefined) {
      console.log("chain data undefined");
      return;
    }

    console.log("chain data: ", chain_data);
    this.id = id;

    let tkns = chain_data.tokens as Array<any>;
    let factories = chain_data.dexes as Array<any>;
    let pools = chain_data.pools as Array<any>;

    if (tkns !== undefined) {
      console.log("has tokens");

      tkns.forEach((tkn) => {
        let new_token = tkn as Token;
        this.insert_token(new_token);
        console.log("inserting token");
      });

      if (factories !== undefined) {
        console.log("has factories");
        factories.forEach((dex) => {
          let new_dex = FactoryBuilder(dex);
          if (new_dex) {
            new_dex.address = getAddress(new_dex.address);
            if (new_dex.address) {
              console.log("new dex: ", new_dex);
              this.factories.push(new_dex);
            } else {
              console.log("invalid dex address: ", dex);
            }
          }
        });
      }

      if (pools !== undefined) {
        this.pools = pools;
        pools.forEach((pool) => {
          let new_pool = PoolBuilder(pool);
          if (new_pool) {
            new_pool.address = getAddress(new_pool.address);
            if (isAddress(new_pool.address)) {
              this.insert_pool(new_pool);
            }
          }
        });
      }
      console.log("created chain ", this.id);
    }
  }

  insert_token(tkn: Token) {
    const tkns = this.tokens;
    if (ethers.isAddress(tkn.address)) {
      tkn.address = getAddress(tkn.address);
      if (!tkns.has(tkn.address)) {
        tkns.set(tkn.address, tkn);
        if (!this.pools_by_token.has(tkn.address)) {
          console.log("inserting token");
          this.pools_by_token.set(tkn.address, []);
        }
      }
    }
  }

  insert_pool(pool: AnyPool) {
    const tkn0 = pool.token0;
    const map = this.pools_by_token;

    const tkn0_pools = map.get(tkn0);
    if (tkn0_pools) {
      if (!tkn0_pools.includes(pool)) {
        tkn0_pools.push(pool);
      }
    } else {
      map.set(tkn0, [pool]);
    }

    const tkn1 = pool.token1;
    const tkn1_pools = map.get(tkn1);
    if (tkn1_pools) {
      if (!tkn1_pools.includes(pool)) {
        tkn1_pools.push(pool);
      }
    } else {
      map.set(tkn1, [pool]);
    }
  }

  merge(other: ChainData) {
    for (const token of other.tokens) {
      if (!this.tokens.has(token[0])) {
        this.tokens.set(token[0], token[1]);
      }
    }
    for (const _pool of other.pools_by_token) {
      const token = _pool[0];
      const pools = _pool[1];

      for (const p of pools) {
        this.insert_pool(p);
      }
    }
  }
}

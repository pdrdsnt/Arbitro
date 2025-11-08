import { AbiCoder, getAddress, isAddress } from "ethers";
import { V2Pool } from "./V2Pool";
import { V3Pool } from "./V3Pool";
import { keccak256 } from "ethers";
import { V4Pool } from "./V4Pool";

export type AnyPool = V2Pool | V3Pool | V4Pool;

export function PoolBuilder(json_data) {
  let version = json_data.version;
  let address = json_data.address;
  if (isAddress(address)) {
    json_data.address = getAddress(address);
    if (version == "v2") {
      let pool: V2Pool = {
        factory: json_data.factory,
        address: json_data.address,
        token0: json_data.token0,
        token1: json_data.token1,
        fee: json_data.fee,
      };
      return pool;
    } else if (version == "v3") {
      let pool: V3Pool = {
        factory: json_data.factory,
        address: json_data.address,
        token0: json_data.token0,
        token1: json_data.token1,
        tick_spacing: json_data.tick_spacing,
        fee: json_data.fee,
      };
      return pool;
    } else if (version == "v4") {
      let t0 = json_data.token0;

      if (isAddress(t0)) {
        t0 = getAddress(t0);
      } else {
        return null;
      }

      let t1 = json_data.token1;

      if (isAddress(t1)) {
        t0 = getAddress(t1);
      } else {
        return null;
      }

      let fee = json_data.fee;
      let tick_spacing = json_data.spacing;
      let hooks = json_data.hooks;

      const encoded = AbiCoder.defaultAbiCoder().encode(
        ["address", "address", "uint24", "int24", "address"],
        [t0, t1, fee, tick_spacing, hooks],
      );

      const poolKey = keccak256(encoded);

      let pool: V4Pool = {
        address: json_data.address,
        key: poolKey,
        token0: t0,
        token1: t1,
        fee: fee,
        tick_spacing: tick_spacing,
        hooks: hooks,
        liquidity: 0n,
      };
      return pool;
    }
  }
}
